use std::io::Write;

use lintropy_core::{Diagnostic, LintropyError, Result, Summary};
use serde::Serialize;

use crate::Reporter;

pub struct JsonReporter<'a> {
    pub writer: Box<dyn Write + 'a>,
}

impl<'a> JsonReporter<'a> {
    pub fn new(writer: Box<dyn Write + 'a>) -> Self {
        Self { writer }
    }
}

#[derive(Serialize)]
struct JsonEnvelope<'a> {
    version: u8,
    diagnostics: &'a [Diagnostic],
    summary: &'a Summary,
}

impl Reporter for JsonReporter<'_> {
    fn report(&mut self, diagnostics: &[Diagnostic], summary: &Summary) -> Result<()> {
        serde_json::to_writer_pretty(
            &mut self.writer,
            &JsonEnvelope {
                version: 1,
                diagnostics,
                summary,
            },
        )
        .map_err(|error| LintropyError::Internal(format!("failed to serialize diagnostics: {error}")))?;
        writeln!(self.writer)?;
        Ok(())
    }
}
