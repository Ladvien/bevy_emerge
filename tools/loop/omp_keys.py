#!/usr/bin/env python3
"""Per-key view and hygiene for omp's stored API keys (``~/.omp/agent/agent.db``).

omp has ``/login`` (adds a key) and ``/logout`` (disables every key of a provider) but no way to list
keys one by one or keep some and disable others. That matters for rotation: on a usage-limit error
omp switches to a sibling stored key before it consults ``retry.fallbackChains``, and quota is per
ACCOUNT, so five keys of one exhausted account are five more failures before the fallback model.
Measured 2026-10-28: the laptop held nine ``ollama-cloud`` keys for two accounts and one dead key.

A key is named by its **digest**, the first 8 hex of SHA-256 of the key. **No command prints a key
or part of one.** Disabling is soft (``disabled_cause``, which omp itself uses for ``/logout``), never
a delete, and ``restore`` undoes exactly what ``keep`` did.

    python3 tools/loop/omp_keys.py [--db PATH] PROVIDER list
    python3 tools/loop/omp_keys.py [--db PATH] ollama-cloud probe
    python3 tools/loop/omp_keys.py [--db PATH] PROVIDER keep DIGEST...
    python3 tools/loop/omp_keys.py [--db PATH] PROVIDER restore ID...
    python3 tools/loop/omp_keys.py [--db PATH] PROVIDER export DIGEST | ssh host python3 omp_keys.py PROVIDER import

Default db: ``$PI_CODING_AGENT_DIR/agent.db``, else ``~/.omp/agent/agent.db``. Stdlib only, Python 3.8+.
"""

import argparse
import datetime
import hashlib
import json
import os
import re
import sqlite3
import subprocess
import sys
import time
import urllib.parse

KEEP_CAUSE_PREFIX = "omp_keys keep "
DIGEST_RE = re.compile(r"^[0-9a-f]{8}$")
OLLAMA_CHAT_URL = "https://ollama.com/api/chat"
PROBE_BODY = {
    "model": "gpt-oss:20b",
    "stream": False,
    "options": {"num_predict": 4},
    "messages": [{"role": "user", "content": "Reply with the word OK."}],
}


class Refusal(Exception):
    """A request the tool will not carry out; main() prints it and exits 2."""


def default_db_path():
    base = os.environ.get("PI_CODING_AGENT_DIR")
    if base:
        return os.path.join(base, "agent.db")
    return os.path.expanduser("~/.omp/agent/agent.db")


def open_db(path):
    # mode=rw: sqlite would otherwise create an empty database at a mistyped path.
    if not os.path.isfile(path):
        raise Refusal("no database at {}".format(path))
    uri = "file:{}?mode=rw".format(urllib.parse.quote(os.path.abspath(path)))
    return sqlite3.connect(uri, uri=True, timeout=30)


def key_of(data):
    try:
        key = json.loads(data).get("key")
    except (ValueError, AttributeError):
        return None
    return key if isinstance(key, str) and key else None


def digest_of_key(key):
    return hashlib.sha256(key.encode("utf-8")).hexdigest()[:8]


def digest_of(data):
    key = key_of(data)
    return digest_of_key(key) if key else None


def api_key_rows(db, provider):
    """(id, data, disabled_cause, created_at) for the provider's api_key rows, by id."""
    return db.execute(
        "SELECT id, data, disabled_cause, created_at FROM auth_credentials"
        " WHERE provider = ? AND credential_type = 'api_key' ORDER BY id",
        (provider,),
    ).fetchall()


def active_blocks(db):
    """credential id -> latest blocked_until_ms still in the future."""
    now_ms = int(time.time() * 1000)
    try:
        found = db.execute(
            "SELECT credential_id, MAX(blocked_until_ms) FROM auth_credential_blocks"
            " WHERE blocked_until_ms > ? GROUP BY credential_id",
            (now_ms,),
        ).fetchall()
    except sqlite3.OperationalError:
        return {}
    return dict(found)


def utc(seconds, fmt="%Y-%m-%dT%H:%M:%SZ"):
    return datetime.datetime.fromtimestamp(seconds, datetime.timezone.utc).strftime(fmt)


def cmd_list(db, provider, args, out, stdin):
    if args:
        raise Refusal("list takes no arguments")
    blocks = active_blocks(db)
    rows = api_key_rows(db, provider)
    for cid, data, cause, created in rows:
        if cause is not None:
            state = "disabled: {}".format(cause)
        elif cid in blocks:
            state = "enabled, blocked until {}".format(utc(blocks[cid] / 1000.0, "%H:%M:%SZ"))
        else:
            state = "enabled"
        out.write("id={} {} {} {}\n".format(cid, digest_of(data) or "no-key", utc(created), state))
    if not rows:
        out.write("no api_key rows for {}\n".format(provider))
    return 0


def curl_config_value(value):
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def probe_key(key):
    """(http code, server text) for a one-token chat; the key reaches curl on stdin, never argv."""
    config = "\n".join(
        [
            "url = " + curl_config_value(OLLAMA_CHAT_URL),
            "header = " + curl_config_value("Authorization: Bearer " + key),
            "header = " + curl_config_value("Content-Type: application/json"),
            "data = " + curl_config_value(json.dumps(PROBE_BODY)),
        ]
    )
    done = subprocess.run(
        ["curl", "-s", "-m", "60", "-w", "\n%{http_code}", "-K", "-"],
        input=config + "\n",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        universal_newlines=True,
    )
    body, _, code = done.stdout.rpartition("\n")
    return code.strip() or "000", body


def cmd_probe(db, provider, args, out, stdin):
    if provider != "ollama-cloud":
        raise Refusal("probe knows only ollama-cloud")
    if args:
        raise Refusal("probe takes no arguments")
    for cid, data, cause, _ in api_key_rows(db, provider):
        key = key_of(data)
        if cause is not None or key is None:
            continue
        code, body = probe_key(key)
        if code == "200":
            out.write("id={} {} HTTP 200 ok\n".format(cid, digest_of_key(key)))
            continue
        try:
            text = json.loads(body).get("error") or body
        except (ValueError, AttributeError):
            text = body
        text = " ".join(str(text).replace(key, "<key>").split())[:110]
        out.write("id={} {} HTTP {} {}\n".format(cid, digest_of_key(key), code, text))
    return 0


def cmd_keep(db, provider, args, out, stdin):
    if not args:
        raise Refusal("keep needs one or more digests")
    if len(set(args)) != len(args):
        raise Refusal("a digest is named twice")
    bad = [d for d in args if not DIGEST_RE.match(d)]
    if bad:
        raise Refusal("not an 8-hex digest: {}".format(" ".join(bad)))
    enabled = [(cid, digest_of(data)) for cid, data, cause, _ in api_key_rows(db, provider) if cause is None]
    enabled_digests = {d for _, d in enabled if d}
    missing = [d for d in args if d not in enabled_digests]
    if missing:
        raise Refusal("no enabled {} key with digest {}".format(provider, " ".join(missing)))
    keep = set(args)
    drop = [cid for cid, d in enabled if d and d not in keep]
    cause = "{}{}: not in the keep-list".format(KEEP_CAUSE_PREFIX, utc(time.time(), "%Y-%m-%d"))
    with db:
        for cid in drop:
            db.execute(
                "UPDATE auth_credentials SET disabled_cause = ?,"
                " updated_at = CAST(strftime('%s','now') AS INTEGER)"
                " WHERE id = ? AND disabled_cause IS NULL",
                (cause, cid),
            )
    out.write("kept {}; disabled ids: {}\n".format(" ".join(args), " ".join(map(str, drop)) or "none"))
    if drop:
        out.write("undo: python3 tools/loop/omp_keys.py {} restore {}\n".format(provider, " ".join(map(str, drop))))
    return 0


def cmd_restore(db, provider, args, out, stdin):
    if not args:
        raise Refusal("restore needs one or more ids")
    try:
        ids = [int(a) for a in args]
    except ValueError:
        raise Refusal("ids are integers: {}".format(" ".join(args)))
    restored, refused = [], []
    with db:
        for cid in ids:
            row = db.execute(
                "SELECT disabled_cause FROM auth_credentials WHERE id = ? AND provider = ? AND credential_type = 'api_key'",
                (cid, provider),
            ).fetchone()
            if row is None:
                refused.append("id {}: no {} api_key row".format(cid, provider))
            elif row[0] is None or not row[0].startswith(KEEP_CAUSE_PREFIX):
                refused.append("id {}: not disabled by keep ({})".format(cid, row[0] or "enabled"))
            else:
                db.execute(
                    "UPDATE auth_credentials SET disabled_cause = NULL,"
                    " updated_at = CAST(strftime('%s','now') AS INTEGER)"
                    " WHERE id = ? AND disabled_cause LIKE 'omp_keys keep %'",
                    (cid,),
                )
                restored.append(cid)
    out.write("restored ids: {}\n".format(" ".join(map(str, restored)) or "none"))
    for line in refused:
        sys.stderr.write("left alone: {}\n".format(line))
    return 2 if refused else 0


def cmd_export(db, provider, args, out, stdin):
    if len(args) != 1 or not DIGEST_RE.match(args[0]):
        raise Refusal("export takes exactly one 8-hex digest")
    if out.isatty():
        raise Refusal("export writes a key; pipe it (to `import`), never to a terminal")
    for _, data, cause, _ in api_key_rows(db, provider):
        if cause is None and digest_of(data) == args[0]:
            out.write(json.dumps({"provider": provider, "credential_type": "api_key", "data": data}) + "\n")
            return 0
    raise Refusal("no enabled {} key with digest {}".format(provider, args[0]))


def cmd_import(db, provider, args, out, stdin):
    if args:
        raise Refusal("import reads one JSON line from stdin and takes no arguments")
    try:
        row = json.loads(stdin.readline())
    except ValueError:
        raise Refusal("stdin is not one JSON line from export")
    if not isinstance(row, dict) or row.get("provider") != provider or row.get("credential_type") != "api_key":
        raise Refusal("not an api_key row for {}".format(provider))
    data = row.get("data")
    digest = digest_of(data) if isinstance(data, str) else None
    if digest is None:
        raise Refusal("the row carries no key")
    for cid, existing, cause, _ in api_key_rows(db, provider):
        if cause is None and digest_of(existing) == digest:
            out.write("already present as id {} ({})\n".format(cid, digest))
            return 0
    with db:
        cur = db.execute(
            "INSERT INTO auth_credentials (provider, credential_type, data) VALUES (?, 'api_key', ?)",
            (provider, data),
        )
    out.write("added id {} ({})\n".format(cur.lastrowid, digest))
    return 0


COMMANDS = {
    "list": cmd_list,
    "probe": cmd_probe,
    "keep": cmd_keep,
    "restore": cmd_restore,
    "export": cmd_export,
    "import": cmd_import,
}


def main(argv=None, out=None, stdin=None):
    out = out if out is not None else sys.stdout
    stdin = stdin if stdin is not None else sys.stdin
    parser = argparse.ArgumentParser(description="Per-key view and hygiene for omp's stored API keys.")
    parser.add_argument("--db", default=None, help="omp's agent.db (default: $PI_CODING_AGENT_DIR/agent.db or ~/.omp/agent/agent.db)")
    parser.add_argument("provider")
    parser.add_argument("command", choices=sorted(COMMANDS))
    parser.add_argument("args", nargs="*")
    ns = parser.parse_args(argv)
    try:
        db = open_db(ns.db or default_db_path())
        try:
            return COMMANDS[ns.command](db, ns.provider, ns.args, out, stdin)
        finally:
            db.close()
    except Refusal as refusal:
        sys.stderr.write("omp_keys: {}\n".format(refusal))
        return 2


if __name__ == "__main__":
    sys.exit(main())
