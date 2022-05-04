#![deny(missing_docs, unsafe_code)]

//! The `complex-code-spotter` tool extracts snippets of code deemed complex
//! according to the following complexity metrics:
//!
//! - Cyclomatic
//! - Cognitive
//!
//! When the value associated to each of the metrics exceeds a preset threshold,
//! a snippet of code is automatically extracted.

mod concurrent;
mod error;
mod metrics;
mod non_utf8;
mod output;
mod snippets;

pub use metrics::Complexity;
pub use output::OutputFormat;
pub use snippets::Snippets;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::available_parallelism;

use globset::{Glob, GlobSet, GlobSetBuilder};
use rust_code_analysis::{get_function_spaces, guess_language, read_file_with_eol};

use concurrent::{ConcurrentRunner, FilesData};
use error::{Error, Result};
use non_utf8::encode_to_utf8;
use snippets::get_code_snippets;

#[derive(Debug)]
struct Parameters {
    output_format: OutputFormat,
    write: bool,
    include: Vec<String>,
    exclude: Vec<String>,
    complexities: Vec<Complexity>,
    thresholds: Vec<usize>,
}

/// Produce snippets of complex code for a source file.
///
/// If no parameters are set, the producer uses *cyclomatic* with a
/// threshold of 15 as default metric.
/// Write on files is disabled by default, but when enabled,
/// *markdown* is the output format.
#[derive(Debug)]
pub struct SnippetsProducer(Parameters);

impl Default for SnippetsProducer {
    fn default() -> Self {
        Self::new()
    }
}

impl SnippetsProducer {
    /// Creates a new `SnippetsProducer` instance.
    pub fn new() -> Self {
        Self(Parameters {
            output_format: OutputFormat::Markdown,
            write: false,
            include: Vec::new(),
            exclude: Vec::new(),
            complexities: vec![Complexity::Cyclomatic],
            thresholds: vec![15],
        })
    }

    /// Sets a glob to include only a certain kind of files
    pub fn include(mut self, include: Vec<String>) -> Self {
        self.0.include = include;
        self
    }

    /// Sets a glob to exclude only a certain kind of files
    pub fn exclude(mut self, exclude: Vec<String>) -> Self {
        self.0.exclude = exclude;
        self
    }

    /// Sets all complexities metric that will be computed.
    pub fn complexities(mut self, complexities: Vec<Complexity>) -> Self {
        self.0.complexities = complexities;
        self
    }

    /// Sets the respective thresholds associated to each complexity metric.
    pub fn thresholds(mut self, thresholds: Vec<usize>) -> Self {
        self.0.thresholds = thresholds.into_iter().map(|v| v.min(100)).collect();
        self
    }

    /// Enables writing on files.
    pub fn enable_write(mut self) -> Self {
        self.0.write = true;
        self
    }

    /// Sets output format.
    pub fn output_format(mut self, output_format: OutputFormat) -> Self {
        self.0.output_format = output_format;
        self
    }

    /// Runs the complex code snippets producer.
    pub fn run<P: AsRef<Path>>(
        self,
        source_path: P,
        output_path: P,
    ) -> Result<Option<Vec<Snippets>>> {
        // Check if output path is a file.
        if output_path.as_ref().is_file() {
            return Err(Error::FormatPath(
                "Output path MUST be a directory".to_string(),
            ));
        }

        // Check that each complexity has an associated threshold.
        if self.0.complexities.len() != self.0.thresholds.len() {
            return Err(Error::Thresholds);
        }

        // Create container for snippets.
        let snippets_context = Arc::new(Mutex::new(Vec::new()));

        let num_jobs = available_parallelism()?.get();
        let cfg = SnippetsConfig {
            complexities: self.0.complexities,
            thresholds: self.0.thresholds,
            snippets: snippets_context.clone(),
        };
        let files_data = FilesData {
            include: Self::mk_globset(self.0.include),
            exclude: Self::mk_globset(self.0.exclude),
            path: source_path.as_ref().to_path_buf(),
        };

        // Extracts snippets concurrently.
        ConcurrentRunner::new(num_jobs, extract_file_snippets).run(cfg, files_data)?;

        // Retrieve snippets.
        let snippets_context = Arc::try_unwrap(snippets_context)
            .map_err(|_| Error::Mutability("Unable to get computed snippets".to_string()))?
            .into_inner()?;

        // If there are no snippets, print a message informing that the code is
        // clean.
        if snippets_context.is_empty() {
            println!("Congratulations! Your code is clean, it does not have any complexity!");
            return Ok(None);
        }

        // Write files.
        if self.0.write {
            self.0
                .output_format
                .write_format(output_path, &snippets_context)?;
        }

        Ok(Some(snippets_context))
    }

    fn mk_globset(elems: Vec<String>) -> GlobSet {
        if elems.is_empty() {
            return GlobSet::empty();
        }
        let mut globset = GlobSetBuilder::new();
        elems.iter().filter(|e| !e.is_empty()).for_each(|e| {
            if let Ok(glob) = Glob::new(e) {
                globset.add(glob);
            }
        });
        globset.build().map_or(GlobSet::empty(), |globset| globset)
    }
}

#[derive(Debug)]
struct SnippetsConfig {
    complexities: Vec<Complexity>,
    thresholds: Vec<usize>,
    snippets: Arc<Mutex<Vec<Snippets>>>,
}

fn extract_file_snippets(source_path: PathBuf, cfg: &SnippetsConfig) -> Result<()> {
    // Read source file an return it as a sequence of bytes.
    let source_file_bytes = read_file_with_eol(&source_path)?.ok_or(Error::WrongContent)?;

    // Convert source code bytes to an utf-8 string.
    // When the conversion is not possible for every bytes,
    // encode all bytes as utf-8.
    let source_file = match std::str::from_utf8(&source_file_bytes) {
        Ok(source_file) => source_file.to_owned(),
        Err(_) => encode_to_utf8(&source_file_bytes)?,
    };

    // Guess which is the language associated to the source file.
    let language = guess_language(source_file.as_bytes(), &source_path)
        .0
        .ok_or(Error::UnknownLanguage)?;

    // Get metrics values for each space which forms the source code.
    let spaces = get_function_spaces(
        &language,
        source_file.as_bytes().to_vec(),
        &source_path,
        None,
    )
    .ok_or(Error::NoSpaces)?;

    // Get code snippets for each metric
    let snippets = get_code_snippets(
        &spaces,
        language.into(),
        source_path,
        source_file.as_ref(),
        &cfg.complexities,
        &cfg.thresholds,
    );

    // If there are snippets, output file/files in the chosen format.
    if let Some(snippets) = snippets {
        cfg.snippets.as_ref().lock()?.push(snippets);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{
        fs::{create_dir_all, read_dir, remove_dir_all, File},
        io::BufReader,
        path::Path,
    };

    use super::*;

    #[derive(Debug)]
    struct Config<'a> {
        source_path: &'a Path,
        output_path: &'a Path,
        compare_path: &'a Path,
        complexities: Vec<Complexity>,
        thresholds: Vec<usize>,
    }

    impl<'a> Config<'a> {
        fn new(source_path: &'a Path, output_path: &'a Path, compare_path: &'a Path) -> Self {
            Self {
                source_path,
                output_path,
                compare_path,
                complexities: Vec::new(),
                thresholds: Vec::new(),
            }
        }

        fn metrics(mut self, complexities: Vec<Complexity>, thresholds: Vec<usize>) -> Self {
            self.complexities = complexities;
            self.thresholds = thresholds;
            self
        }
    }

    fn read_file(path: &Path) -> std::io::Result<serde_json::Value> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let json_file = serde_json::from_reader(reader)?;

        Ok(json_file)
    }

    fn run_comparator(cfg: Config) {
        // Create output directory.
        create_dir_all(cfg.output_path).unwrap();

        // Produce snippets.
        SnippetsProducer::new()
            .complexities(cfg.complexities)
            .thresholds(cfg.thresholds)
            .output_format(OutputFormat::Json)
            .run(cfg.source_path, cfg.output_path)
            .unwrap();

        // Retrieve output and comparison JSON paths.
        let output_paths = read_dir(cfg.output_path).unwrap();
        let compare_paths = read_dir(cfg.compare_path).unwrap();

        // Compare output and comparison JSON files.
        output_paths
            .zip(compare_paths)
            .for_each(|(output, compare)| {
                let json_output = read_file(&output.unwrap().path()).unwrap();
                let compare_output = read_file(&compare.unwrap().path()).unwrap();
                // Catch the panic when test is going to fail.
                let result = std::panic::catch_unwind(|| {
                    assert_eq!(json_output, compare_output);
                });
                if let Err(result) = result {
                    // Remove output directory.
                    remove_dir_all(cfg.output_path).unwrap();
                    // Show the error.
                    panic!("{:?}", result);
                } else {
                    assert!(result.is_ok());
                }
            });

        // Remove output directory.
        remove_dir_all(cfg.output_path).unwrap();
    }

    #[test]
    fn seahorse_high_thresholds() {
        // Define configuration parameters.
        let cfg = Config::new(
            Path::new("data/seahorse/src"),
            Path::new("data/seahorse/output_high"),
            Path::new("data/seahorse/compare_high"),
        )
        .metrics(
            vec![Complexity::Cyclomatic, Complexity::Cognitive],
            vec![15, 15],
        );

        // Run comparator.
        run_comparator(cfg);
    }

    #[test]
    fn seahorse_low_thresholds() {
        let cfg = Config::new(
            Path::new("data/seahorse/src"),
            Path::new("data/seahorse/output_low"),
            Path::new("data/seahorse/compare_low"),
        )
        .metrics(
            vec![Complexity::Cyclomatic, Complexity::Cognitive],
            vec![8, 8],
        );

        run_comparator(cfg);
    }
}
