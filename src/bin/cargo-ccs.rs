use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use complex_code_spotter::{Complexity, OutputFormat, SnippetsProducer};

const fn thresholds_long_help() -> &'static str {
    "Threshold 0 is minimum value, thus no threshold at all.\n\
     Threshold 100 is maximum value, thus each complexity value is not accepted.\n\n\
   Thresholds 0 and 100 are extremes and are generally not recommended"
}

fn possible_values() -> String {
    format!(
        "\n       [possible values: {}, {}]",
        Complexity::all()
            .iter()
            .map(|c| c.to_string().to_lowercase())
            .collect::<Vec<String>>()
            .join(", "),
        Complexity::all()
            .iter()
            .map(|c| format!("{}:threshold", c.to_string().to_lowercase()))
            .collect::<Vec<String>>()
            .join(", ")
    )
}

#[derive(Debug, PartialEq)]
struct CliComplexity(Complexity, usize);

impl std::str::FromStr for CliComplexity {
    type Err = Box<dyn std::error::Error + Send + Sync + 'static>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (complexity, value) = if let Some((complexity, value)) = s.split_once(':') {
            (
                Complexity::from_str(complexity.trim()).map_err(|_| possible_values())?,
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| possible_values())?,
            )
        } else {
            let complexity = Complexity::from_str(s.trim()).map_err(|_| possible_values())?;
            (complexity, complexity.default_threshold())
        };
        Ok(Self(complexity, value))
    }
}

#[derive(Args)]
struct Opts {
    /// Path to a Cargo.toml
    #[clap(long)]
    manifest_path: Option<PathBuf>,
    /// Output path containing the snippets of complex code for each file
    #[clap(value_parser)]
    output_path: PathBuf,
    /// Output the generated paths as they are produced
    #[clap(short, long)]
    verbose: bool,
    /// Glob to include files
    #[clap(long, short = 'I')]
    include: Vec<String>,
    /// Glob to exclude files
    #[clap(long, short = 'X')]
    exclude: Vec<String>,
    /// Output format
    #[clap(long, short = 'O', default_value = OutputFormat::default(), possible_values = OutputFormat::variants())]
    output_format: OutputFormat,
    /// List of complexities metrics and thresholds considered for snippets
    #[clap(long, short, default_values = &["cyclomatic:15","cognitive:15"], long_help = thresholds_long_help())]
    complexities: Vec<CliComplexity>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Complex Code Spotter cargo subcommand
    #[clap(name = "ccs")]
    Ccs(Opts),
}

/// Complex Code Spotter cargo applet
#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    opts: Cmd,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli {
        opts: Cmd::Ccs(opts),
    } = Cli::parse();

    let complexity = opts.complexities.iter().map(|v| v.0).collect();
    let thresholds = opts.complexities.iter().map(|v| v.1).collect();

    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(ref manifest_path) = opts.manifest_path {
        cmd.manifest_path(manifest_path);
    }

    let metadata = cmd.exec()?;
    let source_path = metadata.workspace_packages()[0]
        .manifest_path
        .parent()
        .unwrap()
        .join("src")
        .into_std_path_buf();

    // Enable filter to log the information contained in the lib.
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| {
            if opts.verbose {
                EnvFilter::try_new("debug")
            } else {
                EnvFilter::try_new("info")
            }
        })
        .unwrap();

    // Run tracer.
    tracing_subscriber::fmt()
        .without_time()
        .with_env_filter(filter_layer)
        .with_writer(std::io::stderr)
        .init();

    SnippetsProducer::new()
        .complexities(complexity)
        .thresholds(thresholds)
        .enable_write()
        .output_format(opts.output_format)
        .include(opts.include)
        .exclude(opts.exclude)
        .run(source_path, opts.output_path)?;

    Ok(())
}
