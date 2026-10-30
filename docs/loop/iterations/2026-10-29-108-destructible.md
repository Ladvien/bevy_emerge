# 2026-10-29 family: #108 slice 1, destructible pieces and violence dials

Focus: family · Machine: big · Outcome: shipped (slice)

## Summary
`bevy_carnage` hosted as an optional feature (pin 615c264). `destructible_set` marks a piece (`destructible.ron`) and bakes its fracture once with `fracture_mesh`, cached at `destructible/<piece>.bake.ron` keyed by mesh bytes + cut settings; `destructible_hit` previews a hit on the cached chunks and audits each with `audit_cell`. Dials are `GorePolicy` in `family/bevy_carnage.ron` via `family_set`. Person route: terrain field lines `destructible <piece> on|off` and `hit <piece> x y z r`.

## Proof
`tests/destructible_marks_bake_once.rs` (bake once, cache survives off/on, hit chunks watertight, unmarked refused), `tests/family_carnage_settings.rs`. Clippy -D warnings with and without the feature; lib 362 pass; leaf 17 pass.

## Owed
Viewport drawing of the broken chunks and the cart clip (no display on big); #108 stays open.

## Test diffs
`tests/leaf.rs` ALLOWED: one row for bevy_carnage (owner exception, family). `tests/all.rs`: two `mod` lines (owner exception).
