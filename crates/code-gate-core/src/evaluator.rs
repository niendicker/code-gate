use crate::{GateEvaluation, GateOperator, GateRule, GateStatus, Metric, MissingPolicy};

pub fn evaluate_gate(rule: &GateRule, metrics: &[Metric]) -> GateEvaluation {
    let observed = metrics
        .iter()
        .find(|metric| metric.id == rule.metric && metric.scope == rule.scope)
        .map(|metric| metric.value);

    let status = match observed {
        Some(value) => {
            if compare(value, rule.operator, rule.threshold) {
                GateStatus::Pass
            } else {
                GateStatus::Fail
            }
        }
        None => match rule.missing {
            MissingPolicy::Fail => GateStatus::Fail,
            MissingPolicy::Warn => GateStatus::Warning,
            MissingPolicy::Ignore => GateStatus::Ignored,
        },
    };

    GateEvaluation {
        rule_id: rule.id.clone(),
        metric: rule.metric.clone(),
        scope: rule.scope.clone(),
        observed,
        operator: rule.operator,
        threshold: rule.threshold,
        severity: rule.severity,
        status,
    }
}

fn compare(actual: f64, operator: GateOperator, threshold: f64) -> bool {
    match operator {
        GateOperator::Eq => (actual - threshold).abs() <= f64::EPSILON,
        GateOperator::Ne => (actual - threshold).abs() > f64::EPSILON,
        GateOperator::Gt => actual > threshold,
        GateOperator::Gte => actual >= threshold,
        GateOperator::Lt => actual < threshold,
        GateOperator::Lte => actual <= threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MetricUnit, Severity};

    fn metric(value: f64) -> Metric {
        Metric {
            id: "test.coverage.lines".into(),
            value,
            unit: MetricUnit::Percent,
            source: "test".into(),
            scope: "core".into(),
        }
    }

    fn rule() -> GateRule {
        GateRule {
            id: "core-coverage".into(),
            metric: "test.coverage.lines".into(),
            scope: "core".into(),
            operator: GateOperator::Gte,
            threshold: 95.0,
            severity: Severity::Error,
            missing: MissingPolicy::Fail,
        }
    }

    #[test]
    fn passes_when_threshold_is_met() {
        assert_eq!(
            evaluate_gate(&rule(), &[metric(95.0)]).status,
            GateStatus::Pass
        );
    }

    #[test]
    fn fails_when_threshold_is_not_met() {
        assert_eq!(
            evaluate_gate(&rule(), &[metric(94.9)]).status,
            GateStatus::Fail
        );
    }

    #[test]
    fn honors_missing_policy() {
        let mut gate = rule();
        gate.missing = MissingPolicy::Warn;

        assert_eq!(evaluate_gate(&gate, &[]).status, GateStatus::Warning);
    }
}
