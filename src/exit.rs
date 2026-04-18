//! CLI-side error + exit-code plumbing. Wraps `LintropyError` with the
//! classification required by §7.6.

use crate::core::LintropyError;

/// Exit codes from §7.6.
pub const EXIT_OK: u8 = 0;
pub const EXIT_FAIL_ON: u8 = 1;
pub const EXIT_USER: u8 = 2;
pub const EXIT_INTERNAL: u8 = 3;

/// Top-level CLI error. Carries the exit code the process should return.
#[derive(Debug)]
pub struct CliError {
    message: String,
    exit_code: u8,
}

impl CliError {
    pub fn user(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: EXIT_USER,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: EXIT_INTERNAL,
        }
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<LintropyError> for CliError {
    fn from(err: LintropyError) -> Self {
        let exit_code = match &err {
            LintropyError::ConfigLoad(_)
            | LintropyError::QueryCompile { .. }
            | LintropyError::UnknownCapture { .. }
            | LintropyError::UnknownPredicate { .. }
            | LintropyError::DuplicateRuleId { .. }
            | LintropyError::Yaml(_)
            | LintropyError::Unsupported(_) => EXIT_USER,
            LintropyError::Io(_) | LintropyError::Internal(_) => EXIT_INTERNAL,
        };
        Self {
            message: err.to_string(),
            exit_code,
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::internal(format!("io error: {err}"))
    }
}
