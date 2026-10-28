#!/usr/bin/env python3
"""The changelog with a clip for every visible change: write the clip, add the entry, check it, watch it.

Every shipped feature or enhancement gets one line in the repo's `CHANGELOG.md`, under a `## YYYY-MM-DD`
heading, newest first. A visible change carries a short MP4 and a still of its result; anything else says
`No clip:` and why (an API, a refactor) and names the test that proves it. The owner reads the changelog
to see what was added, so a line that cannot be watched has to say why.

    changelog.py clip  --out docs/changelog/media/<date>-<id>.mp4 (--frames DIR [--fps N] | --video FILE)
                       [--start S] [--duration S] [--poster S]
    changelog.py add   --id ID --title TEXT [--sha SHA] [--date YYYY-MM-DD]
                       (--clip <date>-<id> | --link PATH | --no-clip REASON)
    changelog.py check [--since YYYY-MM-DD]
    changelog.py page  [--out FILE] [--open]

`--repo DIR` (before the command) picks the repository; the default is the git root of the working
directory. Media lives in `<repo>/docs/changelog/media/`, stored by Git LFS (`.gitattributes`), and a
clip there is at most 20 s, 2 MB and 960 px wide, with its still beside it as `<name>.png`. `--link`
points at media that already exists elsewhere in the repo (a backfilled GIF or run video). Stdlib only,
Python 3.8+, plus ffmpeg/ffprobe for `clip` and `check`, and git-lfs for `check`.
"""

import argparse
import datetime
import glob
import html
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import webbrowser

MEDIA_DIR = "docs/changelog/media"
MAX_SECONDS = 20.0
MAX_BYTES = 2 * 1024 * 1024
MAX_STILL_BYTES = 600 * 1024
MAX_WIDTH = 960
CRF_LADDER = (28, 32, 36, 40)
THUMB_WIDTH = 320
DATE_RE = re.compile(r"^## (\d{4}-\d{2}-\d{2})\s*$")
REF_RE = re.compile(r'(?:href|src)="([^"]+)"')
HEADER = "# Changelog\n\nWhat each change added, newest first. A visible change has a clip: click the still to play it. `python3 tools/changelog/changelog.py page --open` plays them all in one page.\n"


class Refusal(Exception):
    """A request the tool will not carry out; main() prints it and exits 2."""


def repo_root(start):
    try:
        out = subprocess.run(["git", "-C", start, "rev-parse", "--show-toplevel"],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True, check=True)
    except subprocess.CalledProcessError:
        raise Refusal("{} is not inside a git repository".format(start))
    return out.stdout.strip()


def changelog_path(repo):
    return os.path.join(repo, "CHANGELOG.md")


def read_lines(repo):
    path = changelog_path(repo)
    if not os.path.isfile(path):
        return HEADER.splitlines(True)
    with open(path) as fh:
        return fh.readlines()


def need(tool):
    if shutil.which(tool) is None:
        raise Refusal("{} is not on PATH".format(tool))


def probe_seconds(path):
    need("ffprobe")
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "json", path],
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True)
    try:
        return float(json.loads(out.stdout)["format"]["duration"])
    except (ValueError, KeyError, TypeError):
        raise Refusal("ffprobe could not read {}".format(path))


# --- clip -----------------------------------------------------------------------------------------

def encode(source_args, out, crf, seek, duration):
    scale = "scale='min({w},iw)':-2:flags=lanczos,format=yuv420p".format(w=MAX_WIDTH)
    cmd = ["ffmpeg", "-loglevel", "error", "-y"] + seek + source_args + ["-t", "{:.3f}".format(duration),
           "-vf", scale, "-an", "-c:v", "libx264", "-preset", "slow", "-crf", str(crf),
           "-movflags", "+faststart", out]
    subprocess.run(cmd, check=True)


def write_poster(source_args, seek_s, out_png):
    scale = "scale='min({w},iw)':-2:flags=lanczos".format(w=MAX_WIDTH)
    cmd = ["ffmpeg", "-loglevel", "error", "-y", "-ss", "{:.3f}".format(seek_s)] + source_args + [
        "-frames:v", "1", "-vf", scale, out_png]
    subprocess.run(cmd, check=True)


def make_clip(out, frames=None, fps=15, video=None, start=0.0, duration=None, poster=None):
    """Write `out` (.mp4) and its still `<base>.png`; return (seconds, bytes) of the clip written."""
    need("ffmpeg")
    if not out.endswith(".mp4"):
        raise Refusal("--out must end in .mp4")
    if (frames is None) == (video is None):
        raise Refusal("give exactly one of --frames and --video")
    if frames is not None:
        pngs = sorted(glob.glob(os.path.join(frames, "*.png")))
        if not pngs:
            raise Refusal("no .png frames in {}".format(frames))
        work = tempfile.mkdtemp(prefix="clip_frames_")
        try:
            for i, src in enumerate(pngs):
                os.symlink(os.path.abspath(src), os.path.join(work, "{:06d}.png".format(i)))
            source = ["-framerate", str(fps), "-i", os.path.join(work, "%06d.png")]
            total = len(pngs) / float(fps)
            return _encode_capped(out, source, total, start, duration, poster)
        finally:
            shutil.rmtree(work, ignore_errors=True)
    if not os.path.isfile(video):
        raise Refusal("no video at {}".format(video))
    total = probe_seconds(video)
    return _encode_capped(out, ["-i", video], total, start, duration, poster)


def _encode_capped(out, source, total, start, duration, poster):
    span = max(0.0, total - start)
    if duration is not None:
        span = min(span, duration)
    if span <= 0:
        raise Refusal("nothing to encode: start {} is past the end ({:.2f} s)".format(start, total))
    if span > MAX_SECONDS:
        raise Refusal("the clip would be {:.1f} s; the cap is {:.0f} s: pick --start and --duration".format(span, MAX_SECONDS))
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    seek = ["-ss", "{:.3f}".format(start)] if start > 0 else []
    for crf in CRF_LADDER:
        encode(source, out, crf, seek, span)
        if os.path.getsize(out) <= MAX_BYTES:
            break
    else:
        os.remove(out)
        raise Refusal("still over {} bytes at crf {}: shorten it".format(MAX_BYTES, CRF_LADDER[-1]))
    # The still is the result, so by default the last frame of the clip, not the first.
    at = (start + span - 0.05) if poster is None else poster
    write_poster(source, max(0.0, at), os.path.splitext(out)[0] + ".png")
    return span, os.path.getsize(out)


# --- add ------------------------------------------------------------------------------------------

def media_html(href, still):
    return '<a href="{}"><img src="{}" width="{}" alt="clip"></a>'.format(
        html.escape(href, quote=True), html.escape(still, quote=True), THUMB_WIDTH)


def entry_line(item_id, title, sha, clip=None, link=None, no_clip=None):
    head = "- **{}**".format(item_id)
    if sha:
        head += " `{}`".format(sha)
    text = "{} {}".format(head, title.strip())
    if clip:
        base = "{}/{}".format(MEDIA_DIR, clip)
        return "{}<br>{}\n".format(text, media_html(base + ".mp4", base + ".png"))
    if link:
        if link.lower().endswith((".gif", ".png", ".jpg", ".jpeg", ".webp")):
            return "{}<br>{}\n".format(text, media_html(link, link))
        poster = os.path.splitext(link)[0] + ".png"
        return "{} [clip]({})\n".format(text, link) if not os.path.exists(poster) else \
            "{}<br>{}\n".format(text, media_html(link, poster))
    return "{} *No clip: {}*\n".format(text, no_clip.strip().rstrip("."))


def insert_entry(lines, date, line):
    heading = "## {}\n".format(date)
    for i, existing in enumerate(lines):
        if existing == heading or existing.rstrip("\n") == heading.rstrip("\n"):
            j = i + 1
            while j < len(lines) and lines[j].strip() == "":
                j += 1
            return lines[:j] + [line] + lines[j:]
    # A new date goes above the first older one, so the file stays newest first.
    for i, existing in enumerate(lines):
        m = DATE_RE.match(existing.rstrip("\n"))
        if m and m.group(1) < date:
            return lines[:i] + [heading, "\n", line, "\n"] + lines[i:]
    tail = list(lines)
    if tail and tail[-1].strip() != "":
        tail.append("\n")
    return tail + [heading, "\n", line]


def cmd_add(repo, args):
    chosen = [x for x in (args.clip, args.link, args.no_clip) if x]
    if len(chosen) != 1:
        raise Refusal("give exactly one of --clip, --link and --no-clip")
    date = args.date or datetime.date.today().isoformat()
    if not re.match(r"^\d{4}-\d{2}-\d{2}$", date):
        raise Refusal("--date is YYYY-MM-DD")
    if args.clip:
        for ext in (".mp4", ".png"):
            path = os.path.join(repo, MEDIA_DIR, args.clip + ext)
            if not os.path.isfile(path):
                raise Refusal("no {}: run `clip` first".format(os.path.relpath(path, repo)))
    if args.link and not os.path.isfile(os.path.join(repo, args.link)):
        raise Refusal("no file at {}".format(args.link))
    if "\n" in args.title:
        raise Refusal("the title is one line")
    line = entry_line(args.id, args.title, args.sha, args.clip, args.link, args.no_clip)
    lines = read_lines(repo)
    with open(changelog_path(repo), "w") as fh:
        fh.writelines(insert_entry(lines, date, line))
    print("added under {}: {}".format(date, line.strip()[:160]))
    return 0


# --- check ----------------------------------------------------------------------------------------

def entries(lines):
    """(date, line number, text) for every bullet under a date heading."""
    date = None
    for n, raw in enumerate(lines, 1):
        line = raw.rstrip("\n")
        m = DATE_RE.match(line)
        if m:
            date = m.group(1)
            continue
        if line.startswith("## ") or line.startswith("# "):
            date = None
            continue
        if date and line.startswith("- "):
            yield date, n, line


def lfs_tracked(repo, rel):
    out = subprocess.run(["git", "-C", repo, "check-attr", "filter", "--", rel],
                         stdout=subprocess.PIPE, universal_newlines=True)
    return out.stdout.strip().endswith(": filter: lfs")


def check(repo, since=None):
    problems = []
    lines = read_lines(repo)
    referenced = set()
    dates = []
    for date, n, text in entries(lines):
        dates.append(date)
        if since and date < since:
            continue
        refs = REF_RE.findall(text) + re.findall(r"\]\(([^)\s]+)\)", text)
        if not refs and "No clip:" not in text:
            problems.append("CHANGELOG.md:{}: no clip and no `No clip:` reason".format(n))
        for ref in refs:
            if re.match(r"^[a-z]+://", ref):
                continue
            path = os.path.join(repo, ref)
            if not os.path.isfile(path):
                problems.append("CHANGELOG.md:{}: {} does not exist".format(n, ref))
                continue
            if ref.startswith(MEDIA_DIR + "/"):
                referenced.add(ref)
    if dates != sorted(dates, reverse=True):
        problems.append("CHANGELOG.md: the date headings are not newest first")
    media = os.path.join(repo, MEDIA_DIR)
    names = sorted(os.listdir(media)) if os.path.isdir(media) else []
    if names and subprocess.run(["git", "lfs", "version"], stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE).returncode != 0:
        problems.append("git-lfs is not installed: a clip committed now would go into git itself")
    for name in names:
        rel = "{}/{}".format(MEDIA_DIR, name)
        path = os.path.join(repo, rel)
        if rel not in referenced and not since:
            problems.append("{}: no changelog line shows it".format(rel))
        if not lfs_tracked(repo, rel):
            problems.append("{}: not stored by Git LFS (.gitattributes)".format(rel))
        if name.endswith(".mp4"):
            size = os.path.getsize(path)
            if size > MAX_BYTES:
                problems.append("{}: {:.2f} MB, over the {:.0f} MB cap".format(rel, size / 1048576.0, MAX_BYTES / 1048576.0))
            seconds = probe_seconds(path)
            if seconds > MAX_SECONDS + 0.5:
                problems.append("{}: {:.1f} s, over the {:.0f} s cap".format(rel, seconds, MAX_SECONDS))
            if not os.path.isfile(os.path.splitext(path)[0] + ".png"):
                problems.append("{}: no still beside it".format(rel))
        elif name.endswith(".png"):
            if os.path.getsize(path) > MAX_STILL_BYTES:
                problems.append("{}: still over {} KB".format(rel, MAX_STILL_BYTES // 1024))
        else:
            problems.append("{}: only .mp4 and its .png belong here".format(rel))
    return problems


def cmd_check(repo, args):
    problems = check(repo, args.since)
    for p in problems:
        print(p)
    if problems:
        print("{} problem(s)".format(len(problems)))
        return 1
    print("the changelog is consistent")
    return 0


# --- page -----------------------------------------------------------------------------------------

def render_page(repo, lines):
    out = ["<!doctype html><meta charset=utf-8><title>{} changelog</title>".format(html.escape(os.path.basename(repo))),
           "<style>body{font:15px system-ui;max-width:1100px;margin:2em auto;padding:0 1em}"
           "li{margin:1.2em 0;list-style:none}video,img{display:block;max-width:960px;margin-top:.4em;border:1px solid #8884}"
           "code{background:#8882;padding:0 .3em}</style>",
           "<h1>{} changelog</h1>".format(html.escape(os.path.basename(repo)))]
    current = None
    for date, _, text in entries(lines):
        if date != current:
            if current:
                out.append("</ul>")
            out.append("<h2>{}</h2><ul>".format(date))
            current = date
        refs = REF_RE.findall(text) + re.findall(r"\]\(([^)\s]+)\)", text)
        words = re.sub(r"<[^>]+>", "", re.sub(r"\[clip\]\([^)]*\)", "", text[2:]))
        words = html.escape(words)
        words = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", words)
        words = re.sub(r"`([^`]+)`", r"<code>\1</code>", words)
        words = re.sub(r"\*(.+?)\*", r"<i>\1</i>", words)
        media = ""
        seen = set()
        for ref in refs:
            if ref in seen:
                continue
            seen.add(ref)
            url = "file://" + os.path.join(repo, ref)
            if ref.endswith(".mp4"):
                still = os.path.splitext(ref)[0] + ".png"
                seen.add(still)
                poster = ' poster="file://{}"'.format(os.path.join(repo, still)) if os.path.isfile(os.path.join(repo, still)) else ""
                media += '<video controls preload=none{} src="{}"></video>'.format(poster, html.escape(url, quote=True))
            elif ref.lower().endswith((".gif", ".png", ".jpg", ".jpeg", ".webp")):
                media += '<img loading=lazy src="{}">'.format(html.escape(url, quote=True))
        out.append("<li>{}{}</li>".format(words, media))
    if current:
        out.append("</ul>")
    return "\n".join(out) + "\n"


def cmd_page(repo, args):
    out = args.out or os.path.join(tempfile.gettempdir(), "changelog-{}.html".format(os.path.basename(repo)))
    with open(out, "w") as fh:
        fh.write(render_page(repo, read_lines(repo)))
    print(out)
    if args.open:
        webbrowser.open("file://" + os.path.abspath(out))
    return 0


def cmd_clip(repo, args):
    seconds, size = make_clip(args.out, frames=args.frames, fps=args.fps, video=args.video,
                              start=args.start, duration=args.duration, poster=args.poster)
    print("{}: {:.1f} s, {:.2f} MB; still {}".format(args.out, seconds, size / 1048576.0,
                                                    os.path.splitext(args.out)[0] + ".png"))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--repo", default=None, help="the repository (default: the git root of the working directory)")
    sub = ap.add_subparsers(dest="command")
    c = sub.add_parser("clip", help="frames or a video -> a capped MP4 and its still")
    c.add_argument("--out", required=True)
    c.add_argument("--frames")
    c.add_argument("--fps", type=int, default=15)
    c.add_argument("--video")
    c.add_argument("--start", type=float, default=0.0)
    c.add_argument("--duration", type=float)
    c.add_argument("--poster", type=float, help="seconds into the source for the still (default: the clip's last frame)")
    a = sub.add_parser("add", help="add one changelog line")
    a.add_argument("--id", required=True)
    a.add_argument("--title", required=True)
    a.add_argument("--sha")
    a.add_argument("--date")
    a.add_argument("--clip", help="<name>: docs/changelog/media/<name>.mp4 and .png")
    a.add_argument("--link", help="existing media elsewhere in the repo, relative to its root")
    a.add_argument("--no-clip", dest="no_clip", help="why there is no clip, and the test that proves the change")
    k = sub.add_parser("check", help="every line has a clip or a reason; every clip is capped and in LFS")
    k.add_argument("--since", help="check only entries on or after YYYY-MM-DD")
    p = sub.add_parser("page", help="write one HTML page that plays every clip")
    p.add_argument("--out")
    p.add_argument("--open", action="store_true")
    args = ap.parse_args(argv)
    if not args.command:
        ap.print_help()
        return 2
    try:
        repo = os.path.abspath(args.repo) if args.repo else repo_root(os.getcwd())
        return {"clip": cmd_clip, "add": cmd_add, "check": cmd_check, "page": cmd_page}[args.command](repo, args)
    except Refusal as refusal:
        sys.stderr.write("changelog: {}\n".format(refusal))
        return 2


if __name__ == "__main__":
    sys.exit(main())
