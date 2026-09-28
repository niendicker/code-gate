mod collector;
mod evaluator;
mod model;

pub use collector::{CargoFeatures, CollectionContext, Collector, CollectorError, CollectorOutput};
pub use evaluator::evaluate_gate;
pub use model::{
    CollectorRun, Finding, GateEvaluation, GateOperator, GateRule, GateStatus, Location, Metric,
    MetricUnit, MissingPolicy, ProjectMetadata, QualityReport, Severity,
};
