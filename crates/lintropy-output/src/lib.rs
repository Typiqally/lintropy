//! Reporter implementations and sink handling for lintropy output.

use std::{
    fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
};

use lintropy_core::{Diagnostic, Result, Summary};
use tempfile::NamedTempFile;

pub mod json;
pub mod text;

pub use json::JsonReporter;
pub use text::TextReporter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    Always,
    Never,
}

impl ColorChoice {
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Always)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    pub fn color_choice(self, no_color: bool, has_output_path: bool, stdout_is_tty: bool) -> ColorChoice {
        if no_color || has_output_path || matches!(self, Self::Json) || !stdout_is_tty {
            ColorChoice::Never
        } else {
            ColorChoice::Always
        }
    }
}

enum OutputTarget {
    Stdout(io::Stdout),
    File {
        temp_file: NamedTempFile,
        destination: PathBuf,
    },
}

pub struct OutputSink {
    target: OutputTarget,
}

impl OutputSink {
    pub fn open(path: Option<&Path>) -> Result<Self> {
        let target = match path {
            Some(path) => {
                let directory = path.parent().unwrap_or_else(|| Path::new("."));
                fs::create_dir_all(directory)?;
                OutputTarget::File {
                    temp_file: NamedTempFile::new_in(directory)?,
                    destination: path.to_path_buf(),
                }
            }
            None => OutputTarget::Stdout(io::stdout()),
        };

        Ok(Self { target })
    }

    pub fn stdout_is_terminal(&self) -> bool {
        matches!(self.target, OutputTarget::Stdout(_)) && io::stdout().is_terminal()
    }

    pub fn has_output_path(&self) -> bool {
        matches!(self.target, OutputTarget::File { .. })
    }

    pub fn writer(&mut self) -> Box<dyn Write + '_> {
        struct SinkWriter<'a>(&'a mut dyn Write);

        impl Write for SinkWriter<'_> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.write(buf)
            }

            fn flush(&mut self) -> io::Result<()> {
                self.0.flush()
            }
        }

        match &mut self.target {
            OutputTarget::Stdout(stdout) => Box::new(SinkWriter(stdout)),
            OutputTarget::File { temp_file, .. } => Box::new(SinkWriter(temp_file)),
        }
    }

    pub fn commit(mut self) -> Result<()> {
        match &mut self.target {
            OutputTarget::Stdout(stdout) => {
                stdout.flush()?;
            }
            OutputTarget::File { .. } => {}
        }

        match self.target {
            OutputTarget::Stdout(_) => Ok(()),
            OutputTarget::File {
                temp_file,
                destination,
            } => temp_file
                .persist(destination)
                .map(|_| ())
                .map_err(|err| err.error.into()),
        }
    }
}

/// Sink that turns diagnostics + summary into human or machine output.
///
/// Every reporter writes through a `Box<dyn Write>` so `--output` can
/// swap the destination without changing reporter types (§7.7 of the spec).
pub trait Reporter {
    /// Emit the diagnostics and summary. Called once per `lintropy check` run.
    fn report(&mut self, diagnostics: &[Diagnostic], summary: &Summary) -> Result<()>;
}
