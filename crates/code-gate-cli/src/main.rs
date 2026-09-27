mod collect;

use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use code_gate_config::{default_policy_toml, load_policy};
use code_gate_core::{GateStatus, evaluate_gate};
use code_gate_report::load_report;

#[derive(Debug, Parser)]
#[command(name = "code-gate")]
#[command(about = "Tool-agnostic quality gates for Rust projects")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a starter quality.toml in the current directory.
    Init {
        #[arg(long, default_value = "quality.toml")]
        path: PathBuf,
    },

    /// Run the collectors enabled in the policy and write a quality report.
    Collect {
        /// Policy file; its directory is the project that gets analyzed.
        #[arg(long, default_value = "quality.toml")]
        config: PathBuf,

        #[arg(long, default_value = "quality.json")]
        output: PathBuf,
    },

    /// Evaluate a canonical quality report against a policy.
    Check {
        #[arg(long, default_value = "quality.toml")]
        config: PathBuf,

        #[arg(long, default_value = "quality.json")]
        report: PathBuf,
    },

    /// Print the Code Gate version.
    Version,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("code-gate error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Init { path } => init(path),
        Command::Collect { config, output } => collect::collect(config, output),
        Command::Check { config, report } => check(config, report),
        Command::Version => {
            println!("code-gate {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn init(path: PathBuf) -> Result<ExitCode> {
    if path.exists() {
        anyhow::bail!("{} already exists", path.display());
    }

    fs::write(&path, default_policy_toml())
        .with_context(|| format!("failed to create {}", path.display()))?;

    println!("created {}", path.display());
    Ok(ExitCode::SUCCESS)
}

fn check(config: PathBuf, report: PathBuf) -> Result<ExitCode> {
    let policy =
        load_policy(&config).with_context(|| format!("failed to load {}", config.display()))?;
    let quality_report =
        load_report(&report).with_context(|| format!("failed to load {}", report.display()))?;

    println!("Code Gate");
    println!("Project: {}", quality_report.project.name);
    println!();

    let mut failed = false;

    for rule in &policy.gates {
        let evaluation = evaluate_gate(rule, &quality_report.metrics);
        let status = match evaluation.status {
            GateStatus::Pass => "PASS",
            GateStatus::Fail => {
                failed = true;
                "FAIL"
            }
            GateStatus::Warning => "WARN",
            GateStatus::Ignored => "SKIP",
        };

        let observed = evaluation
            .observed
            .map(format_value)
            .unwrap_or_else(|| "missing".to_owned());

        println!(
            "{status:>4}  {:<36} observed={:<10} threshold={}",
            evaluation.rule_id,
            observed,
            format_value(evaluation.threshold)
        );
    }

    println!();

    if failed {
        println!("QUALITY GATE: FAILED");
        Ok(ExitCode::from(1))
    } else {
        println!("QUALITY GATE: PASSED");
        Ok(ExitCode::SUCCESS)
    }
}

/// Console rendering of a metric value: at most two decimals, no trailing
/// zeros. Reports keep full precision.
fn format_value(value: f64) -> String {
    let rounded = format!("{value:.2}");
    rounded
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::format_value;

    #[test]
    fn formats_values_compactly() {
        assert_eq!(format_value(0.0), "0");
        assert_eq!(format_value(75.0), "75");
        assert_eq!(format_value(76.66232073011734), "76.66");
        assert_eq!(format_value(80.5), "80.5");
    }
}
