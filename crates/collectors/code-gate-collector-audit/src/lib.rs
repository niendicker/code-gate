use code_gate_core::{CollectionContext, Collector, CollectorError, CollectorOutput};

#[derive(Debug, Default)]
pub struct AuditCollector;

impl Collector for AuditCollector {
    fn name(&self) -> &'static str {
        "audit"
    }

    fn collect(&self, _context: &CollectionContext) -> Result<CollectorOutput, CollectorError> {
        Err(CollectorError::Execution(
            "cargo-audit collector is scaffolded but not wired yet".into(),
        ))
    }
}
