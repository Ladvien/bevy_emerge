//! **A confidence word parses with the type that prints it (E-047, review gap 16).** The mapper kept
//! its own `confidence_word` match beside `Confidence`'s `Display`; two spellings of one word drift.

use bevy_grimoire_core::descriptor::Confidence;

#[test]
fn every_printed_word_parses_back_to_its_confidence() {
    for c in [Confidence::High, Confidence::Medium, Confidence::Low] {
        assert_eq!(c.to_string().parse::<Confidence>(), Ok(c));
    }
}

#[test]
fn a_word_it_does_not_print_is_refused_naming_it() {
    let err = "High".parse::<Confidence>().unwrap_err();
    assert!(err.contains("\"High\""), "{err}");
}
