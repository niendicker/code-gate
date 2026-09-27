use code_gate_core::{CollectionContext, Collector, CollectorError, CollectorOutput};

#[derive(Debug, Default)]
pub struct LlvmCovCollector;

impl Collector for LlvmCovCollector {
    fn name(&self) -> &'static str {
        "llvm-cov"
    }

    fn collect(&self, _context: &CollectionContext) -> Result<CollectorOutput, CollectorError> {
        Err(CollectorError::Execution(
            "cargo-llvm-cov collector is scaffolded but not wired yet".into(),
        ))
    }
}
