use code_gate_core::{CollectionContext, Collector, CollectorError, CollectorOutput};

#[derive(Debug, Default)]
pub struct ClippyCollector;

impl Collector for ClippyCollector {
    fn name(&self) -> &'static str {
        "clippy"
    }

    fn collect(&self, _context: &CollectionContext) -> Result<CollectorOutput, CollectorError> {
        Err(CollectorError::Execution(
            "Clippy collector is scaffolded but not wired yet".into(),
        ))
    }
}
