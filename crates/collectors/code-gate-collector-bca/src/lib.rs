use code_gate_core::{CollectionContext, Collector, CollectorError, CollectorOutput};

#[derive(Debug, Default)]
pub struct BcaCollector;

impl Collector for BcaCollector {
    fn name(&self) -> &'static str {
        "bca"
    }

    fn command(&self, _context: &CollectionContext) -> Option<String> {
        None
    }

    fn collect(&self, _context: &CollectionContext) -> Result<CollectorOutput, CollectorError> {
        Err(CollectorError::NotImplemented(
            "big-code-analysis collector is scaffolded but not wired yet".into(),
        ))
    }
}
