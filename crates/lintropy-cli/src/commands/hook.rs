use std::io::{Read as _, Write};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Instant;

use ignore::gitignore::GitignoreBuilder;
use lintropy_core::{engine, suppress, Config, Diagnostic, Severity, SourceCache, Summary};
use lintropy_output::{json::JsonReporter, Reporter};
use serde_json::Value;

use crate::cli::{HookAgent, HookArgs, HookFormat};
use crate::exit::EXIT_OK;

const EXIT_HOOK_FAIL: u8 = 2;

pub fn run(args: HookArgs) -> Result<u8, crate::exit::CliError> {
    let _agent = resolve_agent(args.agent);

    let stdin = match read_stdin() {
        Ok(stdin) => stdin,
        Err(err) => {
            warn(&args, &format!("failed to read hook payload: {err}"));
            return Ok(EXIT_OK);
        }
    };
    let payload: Value = match serde_json::from_str(&stdin) {
        Ok(payload) => payload,
        Err(err) => {
            warn(&args, &format!("failed to parse hook payload as JSON: {err}"));
            return Ok(EXIT_OK);
        }
    };

    let Some(path) = extract_path(&payload) else {
        return Ok(EXIT_OK);
    };
    let Ok(path) = PathBuf::from(path).canonicalize() else {
        return Ok(EXIT_OK);
    };

    let outcome = match run_with_timeout(args.config.clone(), path) {
        Ok(outcome) => outcome,
        Err(message) => {
            eprintln!("warning: {message}");
            return Ok(EXIT_OK);
        }
    };

    let HookOutcome::Diagnostics {
        diagnostics,
        summary,
    } = outcome
    else {
        return Ok(EXIT_OK);
    };

    if diagnostics.is_empty() {
        return Ok(EXIT_OK);
    }

    if let Err(err) = emit(&args, &diagnostics, &summary) {
        warn(
            &args,
            &format!("failed to write hook output: {}", err.message()),
        );
        return Ok(EXIT_OK);
    }

    if diagnostics
        .iter()
        .any(|diag| diag.severity >= Severity::from(args.fail_on))
    {
        Ok(EXIT_HOOK_FAIL)
    } else {
        Ok(EXIT_OK)
    }
}

#[derive(Debug, Clone, Copy)]
enum ResolvedAgent {
    ClaudeCode,
    Codex,
}

enum HookOutcome {
    Skip,
    Diagnostics {
        diagnostics: Vec<Diagnostic>,
        summary: Summary,
    },
}

fn resolve_agent(agent: HookAgent) -> ResolvedAgent {
    match agent {
        HookAgent::ClaudeCode => ResolvedAgent::ClaudeCode,
        HookAgent::Codex => ResolvedAgent::Codex,
        HookAgent::Auto => {
            if std::env::var_os("CLAUDE_CODE_HOOK").is_some()
                || std::env::var_os("CLAUDE_TOOL_USE").is_some()
                || std::env::var_os("CLAUDE_PROJECT_DIR").is_some()
            {
                ResolvedAgent::ClaudeCode
            } else {
                // TODO(phase-2): add Codex detection once the hook schema is confirmed.
                ResolvedAgent::ClaudeCode
            }
        }
    }
}

fn read_stdin() -> std::io::Result<String> {
    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin)?;
    Ok(stdin)
}

fn extract_path(payload: &Value) -> Option<&str> {
    [
        payload
            .get("tool_input")
            .and_then(|value| value.get("file_path"))
            .and_then(Value::as_str),
        payload
            .get("tool_input")
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str),
        payload.get("file_path").and_then(Value::as_str),
        payload.get("path").and_then(Value::as_str),
        payload.get("filename").and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    .next()
}

fn run_with_timeout(config_path: Option<PathBuf>, path: PathBuf) -> Result<HookOutcome, String> {
    std::thread::scope(|scope| {
        let (tx, rx) = mpsc::channel();
        let display = path.display().to_string();
        scope.spawn(move || {
            let outcome = run_inner(config_path.as_deref(), &path);
            let _ = tx.send(outcome);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(outcome) => Ok(outcome),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                Err(format!("lintropy hook timed out after 10s for {display}"))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err("lintropy hook worker terminated unexpectedly".to_string())
            }
        }
    })
}

fn run_inner(config_path: Option<&Path>, path: &Path) -> HookOutcome {
    let config = match load_config_quiet(config_path) {
        Ok(config) => config,
        Err(_) => return HookOutcome::Skip,
    };

    if is_gitignored(&config.root_dir, path) {
        return HookOutcome::Skip;
    }

    let mut sources = SourceCache::new();
    if let Ok(bytes) = std::fs::read(path) {
        sources.insert(path.to_path_buf(), Arc::<[u8]>::from(bytes));
    }

    let started = Instant::now();
    let diagnostics = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine::run(&config, &[path.to_path_buf()])
    })) {
        Ok(Ok(diagnostics)) => diagnostics,
        Ok(Err(_)) | Err(_) => return HookOutcome::Skip,
    };

    let (diagnostics, _unused) = suppress::filter(diagnostics, &sources);
    let summary = build_summary(&diagnostics, started.elapsed().as_millis() as u64);

    HookOutcome::Diagnostics {
        diagnostics,
        summary,
    }
}

fn load_config_quiet(config_path: Option<&Path>) -> Result<Config, crate::exit::CliError> {
    match config_path {
        Some(path) => Ok(Config::load_from_path(path)?),
        None => {
            let cwd = std::env::current_dir()
                .map_err(|err| crate::exit::CliError::internal(format!("cwd: {err}")))?;
            Ok(Config::load_from_root(&cwd)?)
        }
    }
}

fn is_gitignored(root_dir: &Path, path: &Path) -> bool {
    let gitignore_path = root_dir.join(".gitignore");
    let mut builder = GitignoreBuilder::new(root_dir);
    if gitignore_path.is_file() {
        let _ = builder.add(gitignore_path);
    }
    builder
        .build()
        .map(|gitignore| gitignore.matched(path, false).is_ignore())
        .unwrap_or(false)
}

fn build_summary(diagnostics: &[Diagnostic], duration_ms: u64) -> Summary {
    let mut errors = 0;
    let mut warnings = 0;
    let mut infos = 0;

    for diagnostic in diagnostics {
        match diagnostic.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => infos += 1,
        }
    }

    Summary {
        errors,
        warnings,
        infos,
        files_checked: 1,
        duration_ms,
    }
}

fn emit(
    args: &HookArgs,
    diagnostics: &[Diagnostic],
    summary: &Summary,
) -> Result<(), crate::exit::CliError> {
    let mut stderr = std::io::stderr().lock();
    match args.format {
        HookFormat::Compact => emit_compact(&mut stderr, diagnostics)?,
        HookFormat::Json => emit_json(&mut stderr, diagnostics, summary)?,
    }
    stderr.flush()?;
    Ok(())
}

fn emit_compact(
    mut writer: impl Write,
    diagnostics: &[Diagnostic],
) -> Result<(), crate::exit::CliError> {
    for diagnostic in diagnostics {
        writeln!(
            writer,
            "{}:{}:{} [{}] {}: {}",
            diagnostic.file.display(),
            diagnostic.line,
            diagnostic.column,
            severity_label(diagnostic.severity),
            diagnostic.rule_id,
            diagnostic.message
        )?;
        if let Some(fix) = &diagnostic.fix {
            writeln!(writer, "  help: replace with `{}`", fix.replacement)?;
        }
    }
    Ok(())
}

fn emit_json(
    mut writer: impl Write,
    diagnostics: &[Diagnostic],
    summary: &Summary,
) -> Result<(), crate::exit::CliError> {
    let mut buffer = Vec::new();
    JsonReporter::new(Box::new(&mut buffer)).report(diagnostics, summary)?;
    writer.write_all(&buffer)?;
    Ok(())
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn warn(args: &HookArgs, message: &str) {
    if args.verbose {
        eprintln!("warning: {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_path_uses_spec_precedence() {
        let payload = json!({
            "tool_input": { "path": "first.rs" },
            "file_path": "second.rs",
            "path": "third.rs",
            "filename": "fourth.rs"
        });

        assert_eq!(extract_path(&payload), Some("first.rs"));
    }
}
