use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result, bail};
use code_gate_collector_audit::AuditCollector;
use code_gate_collector_bca::BcaCollector;
use code_gate_collector_clippy::ClippyCollector;
use code_gate_collector_llvm_cov::LlvmCovCollector;
use code_gate_config::{CollectorsConfig, load_policy};
use code_gate_core::{
    CollectionContext, Collector, CollectorError, ProjectMetadata, QualityReport,
};
use code_gate_process::{Invocation, SystemRunner, ToolRunner};
use code_gate_report::write_report;

/// Runs the enabled collectors and writes a canonical quality report.
///
/// Collection never evaluates gates: warnings found by a tool are recorded as
/// evidence and the command still succeeds. Any collector failure aborts
/// without writing a partial report.
pub fn collect(config: PathBuf, output: PathBuf) -> Result<ExitCode> {
    let policy =
        load_policy(&config).with_context(|| format!("failed to load {}", config.display()))?;
    let workspace_root = workspace_root(&config)?;

    let collectors = enabled_collectors(&policy.collectors);
    if collectors.is_empty() {
        bail!(
            "no collectors are enabled in {}; add for example:\n\n[collectors.clippy]\nenabled = true",
            config.display()
        );
    }

    println!("Code Gate collect");
    println!("Project:   {}", policy.project.name);
    println!("Workspace: {}", workspace_root.display());
    println!();

    let context = CollectionContext {
        workspace_root: workspace_root.clone(),
    };
    let mut report = QualityReport {
        project: ProjectMetadata {
            name: policy.project.name.clone(),
            commit: git(
                &workspace_root,
                &["rev-parse", "--verify", "--quiet", "HEAD"],
            ),
            branch: git(
                &workspace_root,
                &["symbolic-ref", "--short", "--quiet", "HEAD"],
            ),
        },
        metrics: Vec::new(),
        findings: Vec::new(),
        gates: Vec::new(),
    };

    for (key, collector) in &collectors {
        let collected = collector.collect(&context).map_err(|error| match error {
            CollectorError::NotImplemented(detail) => anyhow::anyhow!(
                "collector `{key}` is enabled in {} but not implemented yet ({detail}); \
                 set `enabled = false` under [collectors.{key}]",
                config.display()
            ),
            other => anyhow::Error::new(other).context(format!("collector `{key}` failed")),
        })?;

        println!(
            "  OK  {key:<10} {} metrics, {} findings",
            collected.metrics.len(),
            collected.findings.len()
        );
        for metric in &collected.metrics {
            println!(
                "      {:<28} {} ({})",
                metric.id,
                crate::format_value(metric.value),
                metric.scope
            );
        }

        report.metrics.extend(collected.metrics);
        report.findings.extend(collected.findings);
    }

    write_report(&output, &report)
        .with_context(|| format!("failed to write {}", output.display()))?;

    println!();
    println!("wrote {}", output.display());
    Ok(ExitCode::SUCCESS)
}

/// Collectors in execution order, keyed by their `[collectors.<key>]` name.
fn enabled_collectors(config: &CollectorsConfig) -> Vec<(&'static str, Box<dyn Collector>)> {
    let all: [(&'static str, bool, Box<dyn Collector>); 4] = [
        (
            "clippy",
            config.clippy.enabled,
            Box::new(ClippyCollector::new()),
        ),
        (
            "llvm_cov",
            config.llvm_cov.enabled,
            Box::new(LlvmCovCollector::new()),
        ),
        ("audit", config.audit.enabled, Box::new(AuditCollector)),
        ("bca", config.bca.enabled, Box::new(BcaCollector)),
    ];

    all.into_iter()
        .filter(|(_, enabled, _)| *enabled)
        .map(|(key, _, collector)| (key, collector))
        .collect()
}

/// The project analyzed is the directory that holds the policy file.
fn workspace_root(config: &Path) -> Result<PathBuf> {
    let parent = match config.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };

    fs::canonicalize(parent)
        .with_context(|| format!("failed to resolve project directory {}", parent.display()))
}

/// Best-effort Git metadata; absent Git or repository simply yields `None`.
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = SystemRunner
        .run(&Invocation::new("git", root).args(args))
        .ok()?;
    if !output.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use code_gate_config::CollectorConfig;

    use super::*;

    #[test]
    fn only_enabled_collectors_run() {
        let config = CollectorsConfig {
            clippy: CollectorConfig { enabled: true },
            bca: CollectorConfig { enabled: true },
            ..CollectorsConfig::default()
        };

        let keys: Vec<_> = enabled_collectors(&config)
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert_eq!(keys, ["clippy", "bca"]);
    }

    #[test]
    fn nothing_runs_by_default() {
        assert!(enabled_collectors(&CollectorsConfig::default()).is_empty());
    }

    #[test]
    fn workspace_root_is_config_directory() {
        let cwd = fs::canonicalize(".").unwrap();

        assert_eq!(workspace_root(Path::new("quality.toml")).unwrap(), cwd);
        assert_eq!(workspace_root(Path::new("./quality.toml")).unwrap(), cwd);
        assert_eq!(
            workspace_root(Path::new("src/quality.toml")).unwrap(),
            cwd.join("src")
        );
    }
}
