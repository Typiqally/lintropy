use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use lintropy_core::{Diagnostic, Result, Severity, Summary};
use owo_colors::OwoColorize;

use crate::{ColorChoice, Reporter};

pub struct TextReporter<'a> {
    pub writer: Box<dyn Write + 'a>,
    pub color: ColorChoice,
}

impl<'a> TextReporter<'a> {
    pub fn new(writer: Box<dyn Write + 'a>, color: ColorChoice) -> Self {
        Self { writer, color }
    }

    fn render_severity(&self, severity: Severity) -> String {
        let label = match severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };

        if !self.color.is_enabled() {
            return label.to_string();
        }

        match severity {
            Severity::Error => label.red().to_string(),
            Severity::Warning => label.yellow().to_string(),
            Severity::Info => label.cyan().to_string(),
        }
    }

    fn source_line(&self, diagnostic: &Diagnostic) -> Result<String> {
        read_line(&diagnostic.file, diagnostic.line)
    }
}

impl Reporter for TextReporter<'_> {
    fn report(&mut self, diagnostics: &[Diagnostic], summary: &Summary) -> Result<()> {
        for diagnostic in diagnostics {
            let severity = self.render_severity(diagnostic.severity);
            let source_line = self.source_line(diagnostic)?;
            let caret_start = diagnostic.column.saturating_sub(1);
            let caret_len = caret_len(diagnostic, &source_line);

            writeln!(
                self.writer,
                "{severity}[{}]: {}",
                diagnostic.rule_id, diagnostic.message
            )?;
            writeln!(
                self.writer,
                "  --> {}:{}:{}",
                diagnostic.file.display(),
                diagnostic.line,
                diagnostic.column
            )?;
            writeln!(self.writer, "   |")?;
            writeln!(self.writer, "{} | {}", diagnostic.line, source_line)?;
            write!(self.writer, "   | ")?;
            for _ in 0..caret_start {
                write!(self.writer, " ")?;
            }
            write!(self.writer, "{}", "^".repeat(caret_len))?;
            if let Some(fix) = &diagnostic.fix {
                write!(self.writer, " help: replace with `{}`", fix.replacement)?;
            }
            writeln!(self.writer)?;
            writeln!(self.writer, "   |")?;
            writeln!(
                self.writer,
                "   = rule defined in: {}",
                diagnostic.rule_source.display()
            )?;
            writeln!(self.writer, "   = see: lintropy explain {}", diagnostic.rule_id)?;
            if let Some(docs_url) = &diagnostic.docs_url {
                writeln!(self.writer, "   = docs: {docs_url}")?;
            }
            writeln!(self.writer)?;
        }

        writeln!(self.writer, "{}", render_summary(diagnostics, summary))?;
        Ok(())
    }
}

fn read_line(path: &Path, line_number: usize) -> Result<String> {
    let contents = fs::read_to_string(path)?;
    Ok(contents
        .lines()
        .nth(line_number.saturating_sub(1))
        .unwrap_or_default()
        .to_string())
}

fn caret_len(diagnostic: &Diagnostic, source_line: &str) -> usize {
    if diagnostic.end_line == diagnostic.line && diagnostic.end_column > diagnostic.column {
        diagnostic.end_column - diagnostic.column
    } else {
        let line_len = source_line.chars().count();
        line_len.saturating_sub(diagnostic.column.saturating_sub(1)).max(1)
    }
}

fn render_summary(diagnostics: &[Diagnostic], _summary: &Summary) -> String {
    let mut counts = Vec::new();

    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();
    let infos = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Info)
        .count();

    if errors > 0 {
        counts.push(format!("{} {}", errors, pluralize(errors, "error", "errors")));
    }
    if warnings > 0 {
        counts.push(format!(
            "{} {}",
            warnings,
            pluralize(warnings, "warning", "warnings")
        ));
    }
    if infos > 0 {
        counts.push(format!("{} {}", infos, pluralize(infos, "info", "infos")));
    }
    if counts.is_empty() {
        counts.push("0 diagnostics".to_string());
    }

    let files = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.file.clone())
        .collect::<BTreeSet<PathBuf>>()
        .len();
    let autofixes = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.fix.is_some())
        .count();

    let mut summary = format!(
        "Summary: {} across {} {}.",
        counts.join(", "),
        files,
        pluralize(files, "file", "files")
    );

    if autofixes > 0 {
        summary.push_str(&format!(
            " {} {} available — re-run with --fix.",
            autofixes,
            pluralize(autofixes, "autofix", "autofixes")
        ));
    }

    summary
}

fn pluralize<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}
