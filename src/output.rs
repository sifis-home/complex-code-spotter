use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use arg_enum_proc_macro::ArgEnum;
use tracing::debug;

use crate::Snippets;
use crate::{Error, Result};

/// Supported output formats.
#[derive(ArgEnum, Debug, PartialEq)]
pub enum OutputFormat {
    /// Markdown format.
    #[arg_enum(name = "markdown")]
    Markdown,
    /// Html format.
    #[arg_enum(name = "html")]
    Html,
    /// Json format.
    #[arg_enum(name = "json")]
    Json,
    /// Enables all supported output formats.
    #[arg_enum(name = "all")]
    All,
}

impl OutputFormat {
    /// Default output format.
    pub const fn default() -> &'static str {
        "markdown"
    }

    pub(crate) fn write_format<P: AsRef<Path>>(
        &self,
        output_path: P,
        snippets: &[Snippets],
    ) -> Result<()> {
        // Create output filenames.
        let filenames = create_filenames(snippets);

        let output_path = output_path.as_ref();

        match self {
            Self::All => {
                Markdown::write_format(output_path, &filenames, snippets)?;
                Html::write_format(output_path, &filenames, snippets)?;
                Json::write_format(output_path, &filenames, snippets)
            }
            Self::Json => Json::write_format(output_path, &filenames, snippets),
            Self::Markdown => Markdown::write_format(output_path, &filenames, snippets),
            Self::Html => Html::write_format(output_path, &filenames, snippets),
        }
    }
}

fn create_filenames(snippets: &[Snippets]) -> Vec<String> {
    snippets
        .iter()
        .map(|s| {
            s.source_path
                .iter()
                .filter_map(|c| {
                    c.to_str()
                        .map(|s| (![".", "..", ":", "/", "\\"].contains(&s)).then(|| s))
                })
                .flatten()
                .collect::<Vec<&str>>()
                .join("_")
        })
        .collect()
}

trait WriteFormat {
    const EXTENSION: &'static str;
    const DIR: &'static str;

    fn write_format(path: &Path, filenames: &[String], snippets: &[Snippets]) -> Result<()>;

    #[inline(always)]
    fn create_file(path: &Path, extension: &str) -> std::io::Result<File> {
        let final_path = path.with_extension(extension);
        debug!("Creating {:?}", final_path);

        File::create(final_path)
    }

    #[inline(always)]
    fn create_dir(path: &Path, dir: &str) -> Result<PathBuf> {
        let dir = path.join(dir);
        debug!("Creating {:?}", dir);
        create_dir_all(&dir)?;
        Ok(dir)
    }
}

struct Markdown;

impl WriteFormat for Markdown {
    const EXTENSION: &'static str = "md";
    const DIR: &'static str = "markdown";

    fn write_format(path: &Path, filenames: &[String], snippets: &[Snippets]) -> Result<()> {
        let dir = Self::create_dir(path, Self::DIR)?;

        for (filename, snippet) in filenames.iter().zip(snippets) {
            let mut markdown_file = Self::create_file(&dir.join(filename), Self::EXTENSION)?;

            for (complexity_name, all_snippets) in snippet.snippets.iter() {
                writeln!(
                    markdown_file,
                    r#"# {complexity_name}
                {snippets}"#,
                    snippets = all_snippets
                        .iter()
                        .map(|v| {
                            format!(
                                r#"
*complexity:* **{complexity}**

*start line:* **{start_line}**

*end line:* **{end_line}**

```{language}
{text}
```"#,
                                complexity = v.complexity,
                                start_line = v.start_line,
                                end_line = v.end_line,
                                language = snippet.language.name(),
                                text = v.text
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n\n")
                )?;
            }
        }
        Ok(())
    }
}

struct Html;

impl WriteFormat for Html {
    const EXTENSION: &'static str = "html";
    const DIR: &'static str = "html";

    fn write_format(path: &Path, filenames: &[String], snippets: &[Snippets]) -> Result<()> {
        let dir = Self::create_dir(path, Self::DIR)?;

        let mut index_body = Vec::new();

        for (filename, snippet) in filenames.iter().zip(snippets) {
            let final_path = dir.join(filename).with_extension(Self::EXTENSION);
            debug!("Creating {:?}", final_path);

            let mut html_file = File::create(&final_path)?;

            index_body.push(format!(
                "<a href=\"{index_path}\" target=\"_blank\">{index_path}</a><br>",
                index_path = final_path
                    .file_name()
                    .ok_or_else(|| Error::FormatPath(format!(
                        "Error getting filename for {:?}",
                        final_path
                    )))?
                    .to_str()
                    .ok_or_else(|| Error::FormatPath(format!(
                        "Error converting {:?} path to str",
                        final_path
                    )))?
            ));

            let title = path
                .file_name()
                .map_or("Unknown file", |os| os.to_str().unwrap_or("Unknown file"));
            let body = snippet
                .snippets
                .iter()
                .map(|(complexity_name, all_snippets)| {
                    format!(
                        r#"<h1>{complexity_name}</h1>{snippet}"#,
                        snippet = all_snippets
                            .iter()
                            .map(|v| {
                                format!(
                                    r#"
<p>
    complexity: <b>{complexity}</b><br>
    start line: <b>{start_line}</b><br>
    end line: <b>{end_line}</b><br>
    <pre><code>{text}
    </code></pre>
</p>"#,
                                    complexity = v.complexity,
                                    start_line = v.start_line,
                                    end_line = v.end_line,
                                    text = html_escape::encode_text(&v.text),
                                )
                            })
                            .collect::<Vec<String>>()
                            .join("\n\n")
                    )
                })
                .collect::<Vec<String>>()
                .join("\n\n");
            writeln!(
                html_file,
                r#"<!DOCTYPE html>
<html>
<head>
    <title>{title}</title>
</head>
<body>
    {body}
</body>
</html>"#
            )?;
        }

        let mut index_file = File::create(&dir.join("index.html"))?;
        writeln!(
            index_file,
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Index</title>
</head>
<body>
    {index_body}
</body>
</html>"#,
            index_body = index_body.join("\n")
        )?;
        Ok(())
    }
}

struct Json;

impl WriteFormat for Json {
    const EXTENSION: &'static str = "json";
    const DIR: &'static str = "json";

    fn write_format(path: &Path, filenames: &[String], snippets: &[Snippets]) -> Result<()> {
        let dir = Self::create_dir(path, Self::DIR)?;

        for (filename, snippet) in filenames.iter().zip(snippets) {
            let json_file = Self::create_file(&dir.join(filename), Self::EXTENSION)?;

            serde_json::to_writer_pretty(json_file, snippet)?;
        }
        Ok(())
    }
}
