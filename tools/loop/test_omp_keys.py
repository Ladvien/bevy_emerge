#!/usr/bin/env python3
"""omp_keys.py against a throwaway copy of omp's credential schema.

What must hold: no output carries a key, `keep` changes only the provider's other enabled api_key rows,
a refused `keep` changes nothing, `restore` undoes only `keep`, and `export`/`import` move one key
without a terminal ever seeing it. `probe` talks to ollama.com and is proven by running it.

usage: python3 -m unittest tools/loop/test_omp_keys.py
"""
import contextlib
import io
import json
import os
import sqlite3
import sys
import tempfile
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import omp_keys  # noqa: E402

SCHEMA = """
CREATE TABLE auth_credentials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    credential_type TEXT NOT NULL,
    data TEXT NOT NULL,
    disabled_cause TEXT DEFAULT NULL,
    identity_key TEXT DEFAULT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER))
);
CREATE TABLE auth_credential_blocks (
    credential_id INTEGER NOT NULL,
    provider_key TEXT NOT NULL,
    block_scope TEXT NOT NULL DEFAULT '',
    blocked_until_ms INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (credential_id, provider_key, block_scope)
);
"""

# Keys start with letters outside hex, so a key's first 8 characters can never be mistaken for a digest.
KEY_A = "zzalpha-key-0001-qwertyuiop"
KEY_B = "zzbravo-key-0002-asdfghjkl"
KEY_C = "zzcharlie-key-0003-zxcvbnm"
KEY_D = "zzdelta-key-0004-poiuytrewq"
KEY_OTHER = "zzother-provider-key-0005"


def digest(key):
    return omp_keys.digest_of_key(key)


class Tty(io.StringIO):
    def isatty(self):
        return True


class KeysDb(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = self.make_db("a.db")
        db = sqlite3.connect(self.path)
        with db:
            for key in (KEY_A, KEY_B, KEY_C, KEY_D):
                db.execute(
                    "INSERT INTO auth_credentials (provider, credential_type, data, created_at, updated_at)"
                    " VALUES ('ollama-cloud', 'api_key', ?, 1790000000, 1790000000)",
                    (json.dumps({"key": key, "source": "login"}),),
                )
            db.execute(
                "INSERT INTO auth_credentials (provider, credential_type, data, created_at, updated_at)"
                " VALUES ('ollama-cloud', 'oauth', ?, 1790000000, 1790000000)",
                (json.dumps({"access": "zzoauth-token"}),),
            )
            db.execute(
                "INSERT INTO auth_credentials (provider, credential_type, data, created_at, updated_at)"
                " VALUES ('openrouter', 'api_key', ?, 1790000000, 1790000000)",
                (json.dumps({"key": KEY_OTHER, "source": "login"}),),
            )
            future_ms = int(time.time() * 1000) + 3600 * 1000
            db.execute(
                "INSERT INTO auth_credential_blocks VALUES (2, 'ollama-cloud:api_key', '', ?, 0)",
                (future_ms,),
            )
        db.close()

    def tearDown(self):
        self.dir.cleanup()

    def make_db(self, name):
        path = os.path.join(self.dir.name, name)
        db = sqlite3.connect(path)
        db.executescript(SCHEMA)
        db.close()
        return path

    def run_cmd(self, *argv, db=None, out=None, stdin=None):
        out = out if out is not None else io.StringIO()
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            rc = omp_keys.main(["--db", db or self.path] + list(argv), out=out, stdin=stdin or io.StringIO())
        return rc, out.getvalue() if isinstance(out, io.StringIO) else "", err.getvalue()

    def table(self, path=None):
        db = sqlite3.connect(path or self.path)
        rows = db.execute("SELECT * FROM auth_credentials ORDER BY id").fetchall()
        db.close()
        return rows

    def enabled_ollama_keys(self, path=None):
        db = sqlite3.connect(path or self.path)
        rows = db.execute(
            "SELECT data FROM auth_credentials WHERE provider = 'ollama-cloud'"
            " AND credential_type = 'api_key' AND disabled_cause IS NULL"
        ).fetchall()
        db.close()
        return sorted(json.loads(d)["key"] for (d,) in rows)

    def test_list_names_digests_and_blocks_and_never_a_key(self):
        rc, out, _ = self.run_cmd("ollama-cloud", "list")
        self.assertEqual(rc, 0)
        for key in (KEY_A, KEY_B, KEY_C, KEY_D):
            self.assertIn(digest(key), out)
            self.assertNotIn(key, out)
            self.assertNotIn(key[:8], out)
        line_b = [line for line in out.splitlines() if line.startswith("id=2 ")][0]
        self.assertIn("enabled, blocked until ", line_b)
        line_a = [line for line in out.splitlines() if line.startswith("id=1 ")][0]
        self.assertTrue(line_a.endswith(" enabled"), line_a)
        self.assertNotIn("openrouter", out)
        self.assertNotIn(digest(KEY_OTHER), out)

    def test_keep_disables_only_the_providers_other_api_keys(self):
        before = self.table()
        untouched_before = [r for r in before if r[1] != "ollama-cloud" or r[2] != "api_key"]
        rc, out, _ = self.run_cmd("ollama-cloud", "keep", digest(KEY_A), digest(KEY_C))
        self.assertEqual(rc, 0)
        self.assertEqual(self.enabled_ollama_keys(), sorted([KEY_A, KEY_C]))
        after = self.table()
        causes = {r[0]: r[4] for r in after}
        for cid in (2, 4):
            self.assertTrue(causes[cid].startswith("omp_keys keep "), causes[cid])
            self.assertTrue(causes[cid].endswith(": not in the keep-list"), causes[cid])
        self.assertEqual([r for r in after if r[1] != "ollama-cloud" or r[2] != "api_key"], untouched_before)
        self.assertIn("disabled ids: 2 4", out)
        self.assertIn("restore 2 4", out)
        for key in (KEY_A, KEY_B, KEY_C, KEY_D):
            self.assertNotIn(key, out)

    def test_a_refused_keep_changes_nothing(self):
        before = self.table()
        for argv in (
            ["keep"],
            ["keep", digest(KEY_A), "0badf00d"],
            ["keep", digest(KEY_A), digest(KEY_A)],
            ["keep", "nothex!!"],
            ["keep", digest(KEY_OTHER)],
        ):
            rc, _, err = self.run_cmd("ollama-cloud", *argv)
            self.assertEqual(rc, 2, argv)
            self.assertTrue(err, argv)
            self.assertEqual(self.table(), before, argv)

    def test_restore_undoes_keep_and_refuses_a_logout_tombstone(self):
        self.run_cmd("ollama-cloud", "keep", digest(KEY_A))
        db = sqlite3.connect(self.path)
        with db:
            db.execute("UPDATE auth_credentials SET disabled_cause = 'logged out by user' WHERE id = 4")
        db.close()
        rc, out, err = self.run_cmd("ollama-cloud", "restore", "2", "3", "4")
        self.assertEqual(rc, 2)
        self.assertIn("restored ids: 2 3", out)
        self.assertIn("id 4", err)
        self.assertEqual(self.enabled_ollama_keys(), sorted([KEY_A, KEY_B, KEY_C]))
        causes = {r[0]: r[4] for r in self.table()}
        self.assertEqual(causes[4], "logged out by user")

    def test_export_refuses_a_terminal_and_import_round_trips_once(self):
        rc, _, err = self.run_cmd("ollama-cloud", "export", digest(KEY_B), out=Tty())
        self.assertEqual(rc, 2)
        self.assertIn("terminal", err)
        rc, line, _ = self.run_cmd("ollama-cloud", "export", digest(KEY_B))
        self.assertEqual(rc, 0)
        second = self.make_db("b.db")
        rc, out, _ = self.run_cmd("ollama-cloud", "import", db=second, stdin=io.StringIO(line))
        self.assertEqual(rc, 0)
        self.assertIn("added id 1 ({})".format(digest(KEY_B)), out)
        self.assertNotIn(KEY_B, out)
        self.assertEqual(self.enabled_ollama_keys(second), [KEY_B])
        rc, out, _ = self.run_cmd("ollama-cloud", "import", db=second, stdin=io.StringIO(line))
        self.assertEqual(rc, 0)
        self.assertIn("already present as id 1", out)
        self.assertEqual(len(self.table(second)), 1)

    def test_import_rejects_another_providers_row(self):
        _, line, _ = self.run_cmd("openrouter", "export", digest(KEY_OTHER))
        second = self.make_db("b.db")
        rc, _, err = self.run_cmd("ollama-cloud", "import", db=second, stdin=io.StringIO(line))
        self.assertEqual(rc, 2)
        self.assertIn("not an api_key row for ollama-cloud", err)
        self.assertEqual(self.table(second), [])

    def test_a_missing_database_is_not_created(self):
        missing = os.path.join(self.dir.name, "nope.db")
        rc, _, err = self.run_cmd("ollama-cloud", "list", db=missing)
        self.assertEqual(rc, 2)
        self.assertIn("no database", err)
        self.assertFalse(os.path.exists(missing))


if __name__ == "__main__":
    unittest.main()
