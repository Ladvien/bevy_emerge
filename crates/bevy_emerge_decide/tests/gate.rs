use bevy_emerge_decide::*;
use std::collections::BTreeMap;

/// Answers every question with a fixed P(yes), as a recorded exchange would.
struct Fixed(f64);
impl Transport for Fixed {
    fn endpoint(&self) -> String {
        "fixed".into()
    }
    fn send(&self, r: &Request) -> Result<Response, Error> {
        let answers = r.questions.keys().map(|k| (k.clone(), Answer::Noul { noul: self.0 })).collect::<BTreeMap<_, _>>();
        Ok(Response { model: None, answers, usage: None })
    }
}

fn blank(calibrated_at: Option<f64>) -> Check {
    let mut c = Checks::builtin().get("blank_frame").unwrap().clone();
    c.calibration = calibrated_at.map(|agreement| Calibration {
        check: "blank_frame".into(),
        model: "clef-flash".into(),
        set_size: 20,
        agreement,
        threshold: 0.7,
        date: "2026-10-29".into(),
    });
    c
}

fn view() -> Evidence {
    Evidence { images: [("view".to_string(), "data:image/png;base64,AAAA".to_string())].into(), facts: None }
}

#[test]
fn a_calibrated_check_that_finds_a_blank_frame_blocks_and_a_good_capture_does_not() {
    let c = blank(Some(0.95));
    assert!(run_checks([&c], &Fixed(0.97), &view(), 0.9).blocks());
    assert!(!run_checks([&c], &Fixed(0.03), &view(), 0.9).blocks());
}

#[test]
fn an_uncalibrated_check_or_one_below_the_bar_never_blocks() {
    let found = Fixed(0.99);
    for c in [blank(None), blank(Some(0.8))] {
        let r = run_checks([&c], &found, &view(), 0.9);
        assert!(!r.blocks());
        assert!(matches!(&r.outcomes[0], Outcome::Answered { finding, blocking: false, .. } if finding.found));
    }
}

#[test]
fn a_missing_image_is_skipped_and_an_unreachable_model_is_an_error_not_a_pass() {
    let c = blank(Some(0.95));
    let r = run_checks([&c], &Fixed(0.99), &Evidence::default(), 0.9);
    assert!(matches!(&r.outcomes[0], Outcome::Skipped { missing, .. } if missing == "view"));
    let down = Http::new(Config { endpoint: "http://127.0.0.1:1/v1/systemone".into(), timeout_secs: 1, ..Config::default() });
    let r = run_checks([&c], &down, &view(), 0.9);
    assert!(r.has_errors() && !r.blocks());
}
