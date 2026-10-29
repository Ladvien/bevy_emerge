#!/usr/bin/env python3
"""The loop's checker (issue #112): named checks on captures, and a clip check on changelog clips.

  check.py captures DIR [--only a,b] [--bar 0.9] [--replay DIR]
      Runs the editor's checks (bevy_emerge_decide's checks.ron) on DIR: every DIR/<name>.png is the image
      `<name>` (before, after, closeup, view), DIR/facts.json the facts.
  check.py clip ID [--title T] [--replay DIR]
      Runs `clip_matches_title` on docs/changelog/media/ID.png plus three frames of ID.mp4, against the
      changelog line's title (found by ID, or --title).
  check.py calibrate-clips [--out FILE]
      Builds the labelled clip set (every changelog clip with its own title = no, with another clip's title = yes)
      and writes it to FILE (default /tmp/clip-samples.json); with --ask, asks the live model for each and prints
      the calibration to put in clip_checks.ron.

Exit 0: nothing blocking. Exit 1: a CALIBRATED check (agreement >= --bar) found a failure; an uncalibrated check only
prints. Exit 2: a check could not be asked (the endpoint is down), which is not a pass.
Endpoint: EMERGE_DECIDE_ENDPOINT / _MODEL / _KEY (default the local clef-flash). Tests never call a live model.
"""
import argparse, json, os, re, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MEDIA = ROOT / "docs" / "changelog" / "media"
CLIP_CHECKS = Path(__file__).with_name("clip_checks.ron")
LINE = re.compile(r"^- \*\*(?P<id>[^*]+)\*\* `[0-9a-f]+` (?P<title>.*?)(?: \*No clip|<br>|$)")
CLIP = re.compile(r'docs/changelog/media/(?P<clip>[^"]+)\.mp4')


def checker(*args):
    cmd = ["cargo", "run", "-q", "-p", "bevy_emerge_decide", "--example", "check", "--", *args]
    return subprocess.run(cmd, cwd=ROOT).returncode


def titles():
    """(clip stem, line id, title) for each changelog line with a clip."""
    out = []
    for line in (ROOT / "CHANGELOG.md").read_text().splitlines():
        m, c = LINE.match(line), CLIP.search(line)
        if m and c:
            title = re.sub(r"\s*\(recorded on [^)]*\)\s*$", "", m["title"]).strip()
            out.append((c["clip"], m["id"], title))
    return out


def frames(clip, dest):
    """The still and three evenly spaced frames of MEDIA/clip.mp4 in dest; returns the --image arguments."""
    mp4 = MEDIA / f"{clip}.mp4"
    dur = float(subprocess.check_output(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(mp4)], text=True))
    args = ["--image", f"still={MEDIA / (clip + '.png')}"]
    for i, f in enumerate((0.25, 0.5, 0.75), 1):
        out = Path(dest) / f"{clip}-f{i}.png"
        subprocess.check_call(["ffmpeg", "-v", "error", "-y", "-ss", f"{dur * f:.2f}", "-i", str(mp4),
                               "-frames:v", "1", "-vf", "scale=640:-2", str(out)])
        args += ["--image", f"frame{i}={out}"]
    return args


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("captures"); c.add_argument("dir"); c.add_argument("--only")
    k = sub.add_parser("clip"); k.add_argument("id"); k.add_argument("--title")
    z = sub.add_parser("calibrate-clips"); z.add_argument("--out", default="/tmp/clip-samples.json"); z.add_argument("--ask", action="store_true")
    for p in (c, k):
        p.add_argument("--bar", default="0.9"); p.add_argument("--replay")
    a = ap.parse_args()
    extra = (["--replay", a.replay] if getattr(a, "replay", None) else [])
    if a.cmd == "captures":
        return checker("run", "--dir", a.dir, "--bar", a.bar, *(["--only", a.only] if a.only else []), *extra)
    with tempfile.TemporaryDirectory() as tmp:
        if a.cmd == "clip":
            found = {c: t for c, i, t in titles() if a.id in (c, i)}
            title = a.title or next(iter(found.values()), None)
            clip = next(iter(found), a.id)
            if not title:
                sys.exit(f"no changelog line with a clip named {a.id}; pass --title")
            return checker("run", "--checks", str(CLIP_CHECKS), "--bar", a.bar, "--title", title, *frames(clip, tmp), *extra)
        rows = titles()
        samples = []
        for n, (clip, _id, title) in enumerate(rows):
            imgs = dict(a.split("=", 1) for a in frames(clip, "/tmp")[1::2])
            samples.append({"images": imgs, "title": title, "truth": False})
            other = rows[(n + len(rows) // 2) % len(rows)][2]
            if other != title:
                samples.append({"images": imgs, "title": other, "truth": True})
        Path(a.out).write_text(json.dumps(samples, indent=1))
        print(f"{len(samples)} labelled samples from {len(rows)} clips -> {a.out}", file=sys.stderr)
        if a.ask:
            return checker("calibrate", "--checks", str(CLIP_CHECKS), "--check", "clip_matches_title",
                           "--samples", a.out, "--date", os.environ.get("DATE", "unknown"))
        return 0


if __name__ == "__main__":
    sys.exit(main())
