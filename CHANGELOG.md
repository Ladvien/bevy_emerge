# Changelog

## 2026-10-27

- **Behaviour change (`089705f`, #82): a guest `on` a host lands on the host's lid.** `stack::surface_lift` now takes away the `align.y_offset` that `datum` already added, so the lid stands `extent.height` above the host's *base*. Hosts with no `y_offset` are unchanged. A consumer that compensated for the old arithmetic (declaring `ymax`, not `ymax − ymin`) now sinks guests into the lid by the host's `y_offset`. **Migration: declare `ymax − ymin`, not `ymax`.**
- `Extent::height` is documented as base-to-top, independent of `y_offset` (#83).
- `Library::height_disagreements(asset_root, tolerance_m)` compares every surface-offering descriptor's declared height with its glb and lists the ones that disagree, plus those it could not measure (#83).
