use std::collections::HashMap;
use std::path::PathBuf;

use rust_code_analysis::{FuncSpace, LANG};
use serde::Serialize;

use crate::metrics::Complexity;

/// Supported languages.
#[derive(Debug, Serialize)]
pub enum Language {
    /// JavaScript.
    Javascript,
    /// Java.
    Java,
    /// JavaScript variant.
    Mozjs,
    /// Rust.
    Rust,
    /// C/C++.
    Cpp,
    /// Python.
    Python,
    /// TypeScript.
    Typescript,
    /// Tsx incorporates JSX syntax inside TypeScript.
    Tsx,
    /// C variant focused on comments.
    Ccomment,
    /// C/C++ variant focused on macros-
    Preproc,
}

impl From<LANG> for Language {
    fn from(lang: LANG) -> Self {
        match lang {
            LANG::Javascript => Self::Javascript,
            LANG::Java => Self::Java,
            LANG::Mozjs => Self::Mozjs,
            LANG::Rust => Self::Rust,
            LANG::Cpp => Self::Cpp,
            LANG::Python => Self::Python,
            LANG::Typescript => Self::Typescript,
            LANG::Tsx => Self::Tsx,
            LANG::Ccomment => Self::Ccomment,
            LANG::Preproc => Self::Preproc,
        }
    }
}

impl Language {
    /// Retrieves the name of a language.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Javascript => "javascript",
            Self::Java => "java",
            Self::Mozjs => "mozjs",
            Self::Rust => "rust",
            Self::Cpp => "cpp",
            Self::Python => "python",
            Self::Typescript => "typescript",
            Self::Tsx => "tsx",
            Self::Ccomment => "ccomment",
            Self::Preproc => "preproc",
        }
    }
}

/// Snippets data.
#[derive(Debug, Serialize)]
pub struct SnippetData {
    /// Snippet complexity value.
    pub complexity: usize,
    /// Snippet start line.
    pub start_line: usize,
    /// Snippet end line.
    pub end_line: usize,
    /// Snippet text.
    pub text: String,
}

/// Snippets of complex code obtained analyzing each complexity metric and
/// associated to a single source file.
#[derive(Debug, Serialize)]
pub struct Snippets {
    /// Source path.
    pub source_path: PathBuf,
    /// Source language.
    pub language: Language,
    /// Snippets contained in the analyzed source file.
    pub snippets: HashMap<Complexity, Vec<SnippetData>>,
}

impl Snippets {
    fn new(source_path: PathBuf, language: Language, capacity: usize) -> Self {
        Self {
            source_path,
            language,
            snippets: HashMap::with_capacity(capacity),
        }
    }
}

#[inline(always)]
fn save_snippets(
    complexity_type: Complexity,
    complexity: usize,
    start_line: usize,
    end_line: usize,
    text: String,
    snippets: &mut HashMap<Complexity, Vec<SnippetData>>,
) {
    // Create snippet data.
    let snippet_data = SnippetData {
        complexity,
        start_line,
        end_line,
        text,
    };
    // Save snippet data.
    snippets
        .entry(complexity_type)
        .or_insert_with(Vec::new)
        .push(snippet_data);
}

fn obtain_snippets_single_space(
    space: &FuncSpace,
    source_file: &str,
    complexity_thresholds: Vec<(Complexity, usize)>,
    snippets: &mut HashMap<Complexity, Vec<SnippetData>>,
) {
    complexity_thresholds
        .iter()
        .for_each(|(complexity, threshold)| {
            if let Some(complexity_value) = complexity.value(space, *threshold) {
                save_snippets(
                    *complexity,
                    complexity_value,
                    space.start_line,
                    space.end_line,
                    source_file.to_owned(),
                    snippets,
                );
            }
        });
}

fn obtain_snippets(
    spaces: &[FuncSpace],
    source_file: &str,
    complexity_thresholds: Vec<(Complexity, usize)>,
    snippets: &mut HashMap<Complexity, Vec<SnippetData>>,
) {
    // Iter over spaces.
    for space in spaces {
        let complexity_thresholds = complexity_thresholds
            .iter()
            .filter_map(|(complexity, threshold)| {
                complexity.value(space, *threshold).map(|complexity_value| {
                    if complexity_value > *threshold {
                        // Get code snippet from source code.
                        let str_lines: Vec<&str> = source_file
                            .lines()
                            .skip(space.start_line.saturating_sub(1))
                            .take((space.end_line - space.start_line) + 1)
                            .collect();
                        save_snippets(
                            *complexity,
                            complexity_value,
                            space.start_line,
                            space.end_line,
                            str_lines.join("\n"),
                            snippets,
                        );
                    }
                    (*complexity, *threshold)
                })
            })
            .collect::<Vec<(Complexity, usize)>>();

        // Obtain snippets from subspaces which have high complexities values.
        if !complexity_thresholds.is_empty() {
            obtain_snippets(&space.spaces, source_file, complexity_thresholds, snippets);
        }
    }
}

pub(crate) fn get_code_snippets(
    space: &FuncSpace,
    language: Language,
    source_path: PathBuf,
    source_file: &str,
    complexities: &[Complexity],
    thresholds: &[usize],
) -> Option<Snippets> {
    // Delete complexity metrics which are below a specified threshold.
    let complexity_thresholds = complexities
        .iter()
        .zip(thresholds)
        .filter_map(|(complexity, threshold)| {
            complexity
                .value(space, *threshold)
                .map(|_| (*complexity, *threshold))
        })
        .collect::<Vec<(Complexity, usize)>>();

    // Do not extract snippets when the code has lower complexities values.
    if complexity_thresholds.is_empty() {
        return None;
    }

    // Define structure for snippets.
    let mut metrics_snippets = Snippets::new(source_path, language, complexity_thresholds.len());

    // If there is only one space in a file, save the entire source file for
    // each complexity that overcomes a threshold.
    if space.spaces.is_empty() {
        // Snippets are the entire source file when there is only one space.
        obtain_snippets_single_space(
            space,
            source_file,
            complexity_thresholds,
            &mut metrics_snippets.snippets,
        );
    } else {
        // Obtain snippets from the source code using the complexity metrics
        // computed by rust-code-analysis
        obtain_snippets(
            &space.spaces,
            source_file,
            complexity_thresholds,
            &mut metrics_snippets.snippets,
        );
    }

    Some(metrics_snippets)
}
