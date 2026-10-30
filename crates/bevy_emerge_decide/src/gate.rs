use crate::{Check, Error, Evidence, Finding, Transport};
use serde::Serialize;

/// What one check did in a gate run.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    /// The model answered. `blocking` is true when the check is calibrated at the bar and found a failure.
    Answered { finding: Finding, calibrated: bool, blocking: bool },
    /// The captures lack an image or the facts the check needs; it does not answer from less.
    Skipped { check: String, missing: String },
    /// The model could not be asked or answered unusably. Says nothing about the capture.
    Failed { check: String, error: String },
}

/// The result of running checks over one capture.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
}

impl Report {
    /// A calibrated check found a failure: the only result that may block the loop's gate.
    pub fn blocks(&self) -> bool {
        self.outcomes.iter().any(|o| matches!(o, Outcome::Answered { blocking: true, .. }))
    }
    /// Some check could not be asked (unreachable or unusable endpoint): the gate has no verdict for it.
    pub fn has_errors(&self) -> bool {
        self.outcomes.iter().any(|o| matches!(o, Outcome::Failed { .. }))
    }
}

/// Run `checks` over `evidence`. A finding of an uncalibrated check, or one below `bar`, is reported and
/// never blocks.
pub fn run_checks<'a>(
    checks: impl IntoIterator<Item = &'a Check>,
    transport: &dyn Transport,
    evidence: &Evidence,
    bar: f64,
) -> Report {
    let outcomes = checks
        .into_iter()
        .map(|c| match c.run(transport, evidence) {
            Ok(finding) => {
                let blocking = finding.found && c.may_block(bar);
                Outcome::Answered { finding, calibrated: c.calibration.is_some(), blocking }
            }
            Err(Error::MissingEvidence { check, missing }) => Outcome::Skipped { check, missing },
            Err(e) => Outcome::Failed { check: c.name.clone(), error: e.to_string() },
        })
        .collect();
    Report { outcomes }
}
