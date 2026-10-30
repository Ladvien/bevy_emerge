#!/usr/bin/env python3
"""wiggum-chat.py <loop-name> <driver-log>

The live chat of a Wiggum loop, for its Herdr pane (session "loops", one tab per loop). Owner, 2026-10-30:
"expose the actual chat in each of those so I can be watching what's going on in each loop".

Every pass is one `omp -p` run, and omp writes its session as JSONL while it works
(~/.omp/agent/sessions/<cwd>/<time>_<id>.jsonl): each tool call, tool result, thinking block and reply is a
line the moment it happens. This follows the newest session of the loop's clone, prints it as a readable chat,
and moves to the next pass's session when the driver starts one. It also follows the driver log, so the pane
shows each pass's START and END, and reports to Herdr:
  - the loop's state (working while a pass runs, idle between passes, blocked after a failed pass), and
  - the session's title and the latest tool call's intent, as pane metadata for the sidebar.
Read-only: it never writes to the loop, its clone or its session. Python 3.9-compatible (macOS's system python).
"""
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import time

NAME, LOG = sys.argv[1], sys.argv[2]
HOME = os.path.expanduser("~")
PANE = os.environ.get("HERDR_PANE_ID", "")
SESSIONS = os.path.join(HOME, ".omp", "agent", "sessions")


def _herdr():
    for p in (shutil.which("herdr"), HOME + "/.cargo/bin/herdr", HOME + "/.local/bin/herdr", "/opt/homebrew/bin/herdr"):
        if p and os.access(p, os.X_OK):
            return p
    return None


HERDR = _herdr()
RESET, BOLD, DIM, ITAL = "\033[0m", "\033[1m", "\033[2m", "\033[3m"
RED, GREEN, YELLOW, MAGENTA, CYAN, GREY = "\033[31m", "\033[32m", "\033[33m", "\033[35m", "\033[36m", "\033[90m"
PUA = re.compile("[\ue000-\uf8ff\U000f0000-\U0010ffff]")


def clean(s):
    return PUA.sub("", s or "").strip()


def width():
    return max(40, shutil.get_terminal_size((120, 40)).columns)


def out(s=""):
    sys.stdout.write(s + RESET + "\n")
    sys.stdout.flush()


def clip(line, room):
    line = line.replace("\t", "  ")
    return line if len(line) <= room else line[: max(0, room - 1)] + "…"


def block(text, prefix, colour, max_lines):
    lines = text.rstrip().splitlines() or [""]
    room = width() - len(prefix)
    for line in lines[:max_lines]:
        out(colour + prefix + clip(line, room))
    if len(lines) > max_lines:
        out(GREY + prefix + "… +%d more lines" % (len(lines) - max_lines))


# ---- reporting to Herdr --------------------------------------------------------------------------------------

_seq = [0]
_meta = {"title": None, "summary": None, "sent": 0.0, "dirty": False}


def herdr(*args):
    if HERDR and PANE:
        subprocess.run([HERDR] + list(args), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def report_state(state, msg):
    _seq[0] += 1
    herdr("pane", "report-agent", PANE, "--source", "wiggum", "--agent", NAME, "--state", state,
          "--message", clip(msg, 80), "--seq", str(_seq[0]))


def set_meta(title=None, summary=None):
    if title is not None and title != _meta["title"]:
        _meta["title"], _meta["dirty"] = title, True
    if summary is not None and summary != _meta["summary"]:
        _meta["summary"], _meta["dirty"] = summary, True


def flush_meta(force=False):
    if not _meta["dirty"] or (not force and time.time() - _meta["sent"] < 3):
        return
    args = ["pane", "report-metadata", PANE, "--source", "wiggum-chat"]
    if _meta["title"]:
        args += ["--title", clip(_meta["title"], 80)]
    if _meta["summary"]:
        args += ["--token", "summary=" + clip(_meta["summary"], 80)]
    _seq[0] += 1
    herdr(*(args + ["--seq", str(_seq[0])]))
    _meta["sent"], _meta["dirty"] = time.time(), False


# ---- the chat ------------------------------------------------------------------------------------------------

def call_detail(name, a):
    """The one or few lines that say what a tool call does."""
    if not isinstance(a, dict):
        return [str(a)]
    if name == "bash":
        return ["$ " + l for l in str(a.get("command", "")).splitlines()]
    if name in ("read", "write"):
        return [str(a.get("path", ""))]
    if name == "edit":
        return str(a.get("input", "")).splitlines()[:1]
    if name in ("grep", "glob", "find"):
        return [" ".join(str(a[k]) for k in ("pattern", "query", "path") if a.get(k))]
    if name == "eval":
        return [str(a.get("title") or "")] + str(a.get("code", "")).splitlines()[:2]
    if name == "task":
        tasks = a.get("tasks") or []
        return ["%d subagent(s): %s" % (len(tasks), ", ".join(str(t.get("name", "?")) for t in tasks if isinstance(t, dict)))]
    rest = {k: v for k, v in a.items() if k != "i"}
    return [json.dumps(rest, ensure_ascii=False)]


def render(r):
    t = r.get("type")
    if t in ("title", "title_change"):
        title = clean(r.get("title", ""))
        if title:
            out(BOLD + MAGENTA + "## " + title)
            set_meta(title=title)
    elif t == "model_change":
        out(GREY + "   model " + str(r.get("model", "")))
    elif t == "custom_message":
        text = r.get("content") or r.get("text") or json.dumps(r.get("data", ""), ensure_ascii=False)
        if isinstance(text, list):
            text = " ".join(c.get("text", "") for c in text if isinstance(c, dict))
        block(clean(str(text)), "   notice ", GREY, 3)
    elif t == "message":
        m = r.get("message") or {}
        role, content = m.get("role"), m.get("content")
        if isinstance(content, str):
            content = [{"type": "text", "text": content}]
        content = content or []
        if role == "user":
            text = "\n".join(c.get("text", "") for c in content if c.get("type") == "text").strip()
            first = text.splitlines()[0] if text else ""
            out(YELLOW + BOLD + "> " + clip(first, width() - 30) + RESET + YELLOW + "  (%d chars of prompt)" % len(text))
        elif role == "assistant":
            for c in content:
                ct = c.get("type")
                if ct == "thinking" and c.get("thinking", "").strip():
                    block(c["thinking"], "  ~ ", GREY + ITAL, 12)
                elif ct == "text" and c.get("text", "").strip():
                    out()
                    for line in c["text"].rstrip().splitlines():
                        out(BOLD + line)
                    out()
                elif ct == "toolCall":
                    name = c.get("name", "?")
                    intent = clean(c.get("intent") or (c.get("arguments") or {}).get("i", ""))
                    out(CYAN + BOLD + "▸ " + name + RESET + CYAN + ("  " + intent if intent else ""))
                    detail = [l for l in call_detail(name, c.get("arguments") or {}) if l.strip()]
                    for line in detail[:4]:
                        out(CYAN + DIM + "    " + clip(line, width() - 4))
                    if len(detail) > 4:
                        out(CYAN + DIM + "    … +%d lines" % (len(detail) - 4))
                    set_meta(summary=(intent or name))
        elif role == "toolResult":
            err = m.get("isError")
            colour = RED if err else GREY
            for c in content:
                if c.get("type") == "image":
                    out(colour + "    [image %s]" % c.get("mimeType", ""))
                elif c.get("type") == "text":
                    text = c.get("text", "")
                    if text.strip():
                        block(text, "    │ ", colour, 12 if err else 6)


# ---- following files ------------------------------------------------------------------------------------------

class Tail:
    """Complete lines appended to a file since the last call; reopens a truncated or replaced file."""

    def __init__(self, path, from_end=False):
        self.path, self.fh, self.buf, self.ino = path, None, "", None
        self._open(from_end)

    def _open(self, from_end):
        try:
            self.fh = open(self.path, "r", encoding="utf-8", errors="replace")
            self.ino = os.fstat(self.fh.fileno()).st_ino
            if from_end:
                self.fh.seek(0, os.SEEK_END)
        except OSError:
            self.fh = None

    def lines(self):
        if self.fh is None:
            self._open(False)
            if self.fh is None:
                return []
        try:
            st = os.stat(self.path)
            if st.st_ino != self.ino or st.st_size < self.fh.tell():
                self.fh.close()
                self._open(False)
                self.buf = ""
        except OSError:
            return []
        data = self.fh.read()
        if not data:
            return []
        self.buf += data
        parts = self.buf.split("\n")
        self.buf = parts.pop()
        return parts


def repo_of(log_text):
    found = re.findall(r"repo=(\S+)", log_text)
    return found[-1] if found else None


def session_dir(repo):
    if repo:
        rel = os.path.relpath(repo, HOME)
        name = "-" + (rel.replace(os.sep, "-") if not rel.startswith("..") else repo.strip(os.sep).replace(os.sep, "-"))
        d = os.path.join(SESSIONS, name)
        if os.path.isdir(d):
            return d
    for d in glob.glob(os.path.join(SESSIONS, "*")):
        newest = sorted(glob.glob(os.path.join(d, "*.jsonl")))[-1:]
        if not newest:
            continue
        try:
            with open(newest[0], encoding="utf-8", errors="replace") as f:
                for _ in range(3):
                    rec = json.loads(f.readline() or "{}")
                    if rec.get("type") == "session":
                        if repo and rec.get("cwd") == repo:
                            return d
                        break
        except (OSError, ValueError):
            continue
    return None


def driver_line(line, live):
    """Banner and Herdr state for one driver-log line."""
    if "SNAPSHOT" in line:
        return
    msg = line.split(" ", 1)[1] if " " in line else line
    state = None
    if " START command=" in line:
        state, colour = "working", GREEN
    elif re.search(r"END rc=0 ", line) or " idle (" in line or "driver STOP" in line or "driver EXIT" in line:
        state, colour = "idle", GREEN
    elif "END rc=" in line or "TIMED OUT" in line or "pass failed" in line or "REFUSED" in line:
        state, colour = "blocked", RED
    else:
        colour = GREY
    if live:
        out(colour + BOLD + "=== " + clip(line, width() - 4))
    if state:
        return state, msg
    return None


def main():
    sys.stdout.write("\033]0;%s\007" % NAME)
    try:
        history = open(LOG, encoding="utf-8", errors="replace").read()
    except OSError:
        history = ""
    repo = repo_of(history)
    last = None
    for line in history.splitlines():
        got = driver_line(line, False)
        if got:
            last = got
    if last:
        report_state(*last)
    out(BOLD + MAGENTA + "=== %s  (%s)" % (NAME, repo or "repo unknown"))
    dlog = Tail(LOG, from_end=True)
    sdir = session_dir(repo)
    current, stail, checked = None, None, 0.0
    while True:
        for line in dlog.lines():
            got = driver_line(line, True)
            if got:
                report_state(*got)
            repo = repo_of(line) or repo
        now = time.time()
        if now - checked > 2:
            checked = now
            if sdir is None:
                sdir = session_dir(repo)
            newest = sorted(glob.glob(os.path.join(sdir, "*.jsonl")))[-1:] if sdir else []
            if newest and newest[0] != current:
                if stail:
                    for line in stail.lines():
                        try:
                            render(json.loads(line))
                        except ValueError:
                            pass
                current, stail = newest[0], Tail(newest[0])
                out()
                out(MAGENTA + "--- session " + os.path.basename(current))
        if stail:
            for line in stail.lines():
                try:
                    render(json.loads(line))
                except ValueError:
                    pass
        flush_meta()
        time.sleep(0.5)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
