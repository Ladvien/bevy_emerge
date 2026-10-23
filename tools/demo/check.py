#!/usr/bin/env python3
"""Numeric acceptance for a rendered demo GIF, measured from its own frames and events.json.

Checks, each printed with its measured value:
  * every change moment (agent op, click, ⌘Z, Home) changes >= 5% of the scene pixels
    (per-channel diff > 24) between the frame just before it and the frame 0.5 s after it;
  * in every frame >= 40% of scene pixels are not near-black (max channel > 24);
  * no run of frames longer than 1.5 s without motion before the final hold, where a frame
    is "moving" when it differs from the previous frame on >= 0.5% of scene pixels;
  * file size <= --limit-mb.

Usage: check.py <events.json> <out.gif> [--limit-mb 3]
"""
import argparse
import json
import os
import sys

import numpy as np
from PIL import Image, ImageSequence

CHANGE_DIFF = 24
CHANGE_MIN = 0.05
SCENE_MIN = 0.40
STATIC_MAX_S = 1.5
MOTION_MIN = 0.005
CAPTION_BAR = 56
VIEWPORT_X = (470, 950)  # the 3D viewport's logical x range in the editor window (see storyboards' crop)


def change_moments(rec):
    """(kind, t) for agent ops, clicks, ⌘Z and Home, from the recorded chunks and events."""
    moments = []
    for c in rec.get("chunks", []):
        if c.get("operation_result") is not None:
            moments.append(("agent", c["start"] + (c.get("operation_applied_t_s") or 0.0)))
    cursors = [e["t"] for e in rec["events"] if e["type"] == "cursor"]
    for e in rec["events"]:
        if e["type"] == "click" and VIEWPORT_X[0] <= e["x"] <= VIEWPORT_X[1]:
            # a press that starts a drag (snip box) is followed at once by cursor moves: not a placement
            if not any(0 < t - e["t"] <= 0.1 for t in cursors):
                moments.append(("place", e["t"]))
        elif e["type"] == "key":
            keys = e.get("keys", [])
            if "KeyZ" in keys and "SuperLeft" in keys:
                moments.append(("undo", e["t"]))
            elif "Home" in keys:
                moments.append(("home", e["t"]))
    return sorted(moments, key=lambda m: m[1])


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("events")
    ap.add_argument("gif")
    ap.add_argument("--limit-mb", type=float, default=3.0)
    args = ap.parse_args()

    with open(args.events) as fh:
        rec = json.load(fh)
    fps = rec["fps"]
    frames = [np.asarray(f.convert("RGB"), dtype=np.int16)
              for f in ImageSequence.Iterator(Image.open(args.gif))]
    n = len(frames)
    h = frames[0].shape[0]
    scene_h = h - CAPTION_BAR
    scenes = [f[:scene_h] for f in frames]
    ok = True

    def diff_fraction(i, j):
        return float((np.abs(scenes[i] - scenes[j]).max(axis=2) > CHANGE_DIFF).mean())

    # 1. Scene fraction in every frame.
    worst_scene = min(float((s.max(axis=2) > 24).mean()) for s in scenes)
    scene_ok = worst_scene >= SCENE_MIN
    ok &= scene_ok
    print(f"scene fraction (min over frames): {worst_scene * 100:.1f}% "
          f"(need >= {SCENE_MIN * 100:.0f}%) {'PASS' if scene_ok else 'FAIL'}")

    # 2. Change moments: frame before the moment vs 0.5 s after it.
    for kind, t in change_moments(rec):
        before = min(n - 1, max(0, round((t - 1.0 / fps) * fps)))
        after = min(n - 1, max(0, round((t + 0.5) * fps)))
        frac = diff_fraction(before, after)
        passed = frac >= CHANGE_MIN
        ok &= passed
        print(f"change {kind:6s} at {t:6.2f}s: {frac * 100:5.2f}% of scene pixels changed "
              f"(need >= {CHANGE_MIN * 100:.0f}%) {'PASS' if passed else 'FAIL'}")

    # 3. Static runs before the final hold.
    moving = [False] + [diff_fraction(i - 1, i) >= MOTION_MIN for i in range(1, n)]
    last_move = 0
    worst = 0.0
    for i in range(1, n):
        if moving[i]:
            worst = max(worst, (i - last_move) / fps)
            last_move = i
    tail = (n - 1 - last_move) / fps
    static_ok = worst <= STATIC_MAX_S
    ok &= static_ok
    print(f"longest static run before the final hold: {worst:.2f}s "
          f"(need <= {STATIC_MAX_S}s) {'PASS' if static_ok else 'FAIL'}; final hold {tail:.2f}s")

    # 4. Size.
    mb = os.path.getsize(args.gif) / 1e6
    size_ok = mb <= args.limit_mb
    ok &= size_ok
    print(f"size: {mb:.2f} MB (limit {args.limit_mb} MB) {'PASS' if size_ok else 'FAIL'}")

    print("RESULT:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
