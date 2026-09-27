//! Minimal, testable execution of external analysis tools.
//!
//! Collectors describe *what* to run as an [`Invocation`] and receive raw
//! [`ToolOutput`]. They never build shell strings: arguments are passed to the
//! program as-is through [`std::process::Command`].

use std::{
    ffi::OsString,
    io,
    path::PathBuf,
    process::{Command, Stdio},
};

use thiserror::Error;

/// A single external program invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub current_dir: PathBuf,
}

impl Invocation {
    pub fn new(program: impl Into<OsString>, current_dir: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: current_dir.into(),
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Human-readable command line, for diagnostics only.
    pub fn display(&self) -> String {
        std::iter::once(&self.program)
            .chain(&self.args)
            .map(|part| part.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Raw result of a finished process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolOutput {
    /// `None` when the process was terminated by a signal.
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ToolOutput {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

pub trait ToolRunner {
    /// Runs the invocation to completion.
    ///
    /// A non-zero exit code is *not* an error at this level: many analyzers
    /// report findings through their exit status. Interpreting it is the
    /// collector's responsibility.
    fn run(&self, invocation: &Invocation) -> Result<ToolOutput, RunError>;
}

/// Runs real processes on the host.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl ToolRunner for SystemRunner {
    fn run(&self, invocation: &Invocation) -> Result<ToolOutput, RunError> {
        // Spawning with a missing working directory also yields `NotFound`,
        // which would otherwise be misreported as a missing program.
        if !invocation.current_dir.is_dir() {
            return Err(RunError::WorkingDirectory {
                path: invocation.current_dir.clone(),
            });
        }

        let output = Command::new(&invocation.program)
            .args(&invocation.args)
            .current_dir(&invocation.current_dir)
            .stdin(Stdio::null())
            .output()
            .map_err(|source| {
                let program = invocation.program.to_string_lossy().into_owned();
                if source.kind() == io::ErrorKind::NotFound {
                    RunError::NotFound { program }
                } else {
                    RunError::Spawn { program, source }
                }
            })?;

        Ok(ToolOutput {
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

#[derive(Debug, Error)]
pub enum RunError {
    #[error("`{program}` was not found; is it installed and on PATH?")]
    NotFound { program: String },

    #[error("working directory does not exist: {}", path.display())]
    WorkingDirectory { path: PathBuf },

    #[error("failed to start `{program}`: {source}")]
    Spawn {
        program: String,
        #[source]
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_program() {
        let invocation = Invocation::new("code-gate-no-such-program", ".");

        assert!(matches!(
            SystemRunner.run(&invocation),
            Err(RunError::NotFound { .. })
        ));
    }

    #[test]
    fn reports_missing_working_directory() {
        let invocation = Invocation::new("cargo", "./code-gate-no-such-directory");

        assert!(matches!(
            SystemRunner.run(&invocation),
            Err(RunError::WorkingDirectory { .. })
        ));
    }

    #[test]
    fn captures_output_and_exit_code() {
        // `cargo` is always available where this crate's tests run.
        let invocation = Invocation::new("cargo", ".").arg("--version");
        let output = SystemRunner.run(&invocation).unwrap();

        assert!(output.success());
        assert!(output.stdout.starts_with(b"cargo "));
    }

    #[test]
    fn displays_command_line() {
        let invocation = Invocation::new("cargo", ".").args(["clippy", "--workspace"]);

        assert_eq!(invocation.display(), "cargo clippy --workspace");
    }
}
