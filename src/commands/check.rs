//! `lintropy check` — the centerpiece subcommand.
//!
//! Pipeline: resolve config → walk paths → `engine::run` →
//! `suppress::filter` → reporter → optional autofix. Exit code follows §7.6.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::core::{engine, fix, suppress, Config, Diagnostic, Severity, SourceCache, Summary};
use crate::output::{
    json::JsonReporter, text::TextReporter, OutputFormat as LoFormat, OutputSink, Reporter,
};

use crate::cli::{CheckArgs, OutputFormat};
use crate::commands::{load_config, print_warnings};
use crate::exit::{CliError, EXIT_FAIL_ON, EXIT_OK};
use crate::walk;

pub fn run(args: CheckArgs) -> Result<u8, CliError> {
    let config = load_check_config(&args)?;
    print_warnings(&config);

    let files = walk::expand(&args.paths)?;
    let files_checked = files.len();

    let mut sources = SourceCache::new();
    for path in &files {
        if let Ok(bytes) = std::fs::read(path) {
            sources.insert(path.clone(), Arc::<[u8]>::from(bytes));
        }
    }

    let started = Instant::now();
    let diagnostics = run_engine(&config, &files)?;
    let (survivors, _unused) = suppress::filter(diagnostics, &sources);
    let duration_ms = started.elapsed().as_millis() as u64;

    let summary = build_summary(&survivors, files_checked, duration_ms);

    if args.fix_dry_run {
        emit_dry_run(&survivors)?;
        return Ok(EXIT_OK);
    }

    if !args.quiet {
        emit_report(&args, &survivors, &summary)?;
    }

    if args.fix {
        let report = fix::apply(&survivors)?;
        println!(
            "Applied {} {} across {} {}{}.",
            report.applied,
            pluralize(report.applied, "fix", "fixes"),
            report.files.len(),
            pluralize(report.files.len(), "file", "files"),
            if report.skipped.is_empty() {
                String::new()
            } else {
                format!(" ({} skipped due to overlap)", report.skipped.len())
            }
        );
    }

    Ok(exit_code_for(&survivors, config.settings.fail_on))
}

fn load_check_config(args: &CheckArgs) -> Result<Config, CliError> {
    if let Some(path) = args.config.as_deref() {
        return Ok(Config::load_from_path(path)?);
    }

    if let Some(start) = args.paths.first() {
        return Ok(Config::load_from_root(start)?);
    }

    load_config(None)
}

fn run_engine(
    config: &crate::core::Config,
    files: &[PathBuf],
) -> Result<Vec<Diagnostic>, CliError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine::run(config, files)))
        .map_err(|_| CliError::internal("engine panicked"))?
        .map_err(CliError::from)
}

fn emit_report(
    args: &CheckArgs,
    diagnostics: &[Diagnostic],
    summary: &Summary,
) -> Result<(), CliError> {
    let mut sink = OutputSink::open(args.output.as_deref())?;
    let lo_format = match args.format {
        OutputFormat::Text => LoFormat::Text,
        OutputFormat::Json => LoFormat::Json,
    };
    let color = lo_format.color_choice(
        args.no_color,
        sink.has_output_path(),
        sink.stdout_is_terminal(),
    );

    {
        let writer = sink.writer();
        let mut reporter: Box<dyn Reporter> = match args.format {
            OutputFormat::Text => Box::new(TextReporter::new(writer, color)),
            OutputFormat::Json => Box::new(JsonReporter::new(writer)),
        };
        reporter.report(diagnostics, summary)?;
    }
    sink.commit()?;
    Ok(())
}

fn emit_dry_run(diagnostics: &[Diagnostic]) -> Result<(), CliError> {
    let diff = fix::dry_run(diagnostics)?;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(diff.as_bytes())?;
    handle.flush()?;
    Ok(())
}

fn build_summary(diagnostics: &[Diagnostic], files_checked: usize, duration_ms: u64) -> Summary {
    let mut errors = 0;
    let mut warnings = 0;
    let mut infos = 0;
    for d in diagnostics {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => infos += 1,
        }
    }
    Summary {
        errors,
        warnings,
        infos,
        files_checked,
        duration_ms,
    }
}

fn exit_code_for(diagnostics: &[Diagnostic], fail_on: Severity) -> u8 {
    if diagnostics.iter().any(|d| d.severity >= fail_on) {
        EXIT_FAIL_ON
    } else {
        EXIT_OK
    }
}

fn pluralize<'a>(n: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if n == 1 {
        singular
    } else {
        plural
    }
}
