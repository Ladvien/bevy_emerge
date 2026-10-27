//! **Every surface class the project declares gets a two-way mount, not only the first** — E-060.
//!
//! # The defect
//!
//! `Mount::OnSurfaceOrFloor` says *this may go on a support top **or** on the floor*, and
//! `mount_options` offered it for `surfaces.first()` alone. `assets/emerge/vocab.ron` declares three
//! classes — `support`, `worktop`, `shelf` — so a dressing set authored in the editor could be cycled
//! onto `on support or floor` and nothing else: a piece that belongs on a **shelf** or the floor had
//! no rung, and the VLM labeler's prompt offered the shape for one class while refusing every other.
//! The only workaround was hand-editing the file, which is the one thing the `M` cycle exists to make
//! unnecessary.
//!
//! # What it cost, and why it is worth the length
//!
//! The old comment justified the limit by list length: *"one `or the floor` entry per class would put
//! the list's length at the vocabulary's twice."* That is true and it was the wrong trade — the list is
//! a cycle an author taps through, and an entry that does not exist cannot be reached at any price.
//! With the shipped three classes the list goes from 15 rungs to 17, which is a tap or two; the
//! alternative is a mount that cannot be authored at all.
//!
//! # What this pins
//!
//! 1. Every declared class has its own `OnSurfaceOrFloor` entry.
//! 2. Each entry carries **its own** class — a list that offered three entries all naming `support`
//!    would satisfy a count and be useless.
//! 3. The order stays readable: the two-way entries follow the plain surface entries, so `M` walks
//!    *class by class* and then the same classes again with `or floor`.
//! 4. A project declaring no surface class still offers no two-way mount — the entry is derived from
//!    the vocabulary, never invented.
//! 5. A one-class project is unchanged, so the common case does not pay for the fix.

use bevy_emerge_core::descriptor::{Mount, mount_label, mount_options};

/// The classes `assets/emerge/vocab.ron` declares, in its order.
fn shipped() -> Vec<String> {
    ["support", "worktop", "shelf"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

fn two_way(options: &[Mount]) -> Vec<String> {
    options
        .iter()
        .filter_map(|m| match m {
            Mount::OnSurfaceOrFloor { class } => Some(class.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn every_surface_class_has_a_two_way_mount() {
    let options = mount_options(&shipped());
    assert_eq!(
        two_way(&options),
        shipped(),
        "every class the project declares must be offerable as `on <class> or floor`, and each \
         entry must name its own class. Offered list: {:?}",
        options.iter().map(|m| mount_label(Some(m))).collect::<Vec<_>>()
    );
}

/// **The two-way rungs follow the plain ones**, so the cycle reads as two passes over the vocabulary
/// rather than an interleaving an author has to decode.
#[test]
fn the_two_way_rungs_come_after_the_classes_they_lift_from() {
    let options = mount_options(&shipped());
    let last_plain = options
        .iter()
        .rposition(|m| matches!(m, Mount::OnSurface { .. }))
        .unwrap_or_else(|| panic!("no plain surface mount in {options:?}"));
    let first_two_way = options
        .iter()
        .position(|m| matches!(m, Mount::OnSurfaceOrFloor { .. }))
        .unwrap_or_else(|| panic!("no two-way mount in {options:?}"));
    assert!(
        first_two_way > last_plain,
        "the two-way entries must follow the classes they lift from: {options:?}"
    );
}

/// A project with no surface vocabulary offers no surface mount and therefore no two-way one —
/// the entry comes from the classes rather than being invented beside them.
#[test]
fn no_classes_means_no_two_way_mount() {
    let options = mount_options(&[]);
    assert!(
        two_way(&options).is_empty(),
        "a project declaring no surface class must offer no `or floor` mount: {options:?}"
    );
}

/// **One class is the common case**, and it must stay exactly as it was: one plain entry and one
/// two-way entry, both naming that class.
#[test]
fn a_one_class_project_is_unchanged() {
    let options = mount_options(&["worktop".to_owned()]);
    assert_eq!(two_way(&options), ["worktop"]);
    let plain: Vec<&str> = options
        .iter()
        .filter_map(|m| match m {
            Mount::OnSurface { class } => Some(class.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(plain, ["worktop"]);
}
