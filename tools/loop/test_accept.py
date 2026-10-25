#!/usr/bin/env python3
"""The acceptance script's arithmetic, tested without an editor.

`reached`, `of` and `first_failure` are what the loop grades itself on, so they get a test that does
not need a build. Everything else in accept.py is I/O against a live editor and is proven by running
it — a mock of the editor would only prove the mock.

usage: python3 -m unittest tools/loop/test_accept.py
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from accept import summarize, steps_ok, _skip  # noqa: E402


def step(step_id, status):
    return {"id": step_id, "status": status, "ms": None, "detail": ""}


class Summarize(unittest.TestCase):
    def test_every_step_passing_is_a_full_reached(self):
        steps = [step("tools", "pass"), step("make_map", "pass"), step("open_map", "pass")]
        self.assertEqual(summarize(steps), {"reached": 3, "of": 3, "first_failure": None})

    def test_a_skip_is_not_a_pass_and_not_a_failure(self):
        # The distinction the whole report rests on: a step nobody could attempt has not been shown
        # to be broken, and counting it as broken blames the wrong thing.
        steps = [step("tools", "pass"), step("place", "skip"), step("accept", "skip")]
        self.assertEqual(summarize(steps), {"reached": 1, "of": 3, "first_failure": None})

    def test_the_first_failure_is_the_first_in_the_order_given(self):
        steps = [step("tools", "pass"), step("make_map", "fail"), step("place", "fail")]
        self.assertEqual(summarize(steps)["first_failure"], "make_map")

    def test_a_skip_ahead_of_a_failure_does_not_become_the_first_failure(self):
        steps = [step("tools", "pass"), step("open_map", "skip"), step("place", "fail")]
        self.assertEqual(summarize(steps)["first_failure"], "place")

    def test_an_empty_list_is_zero_of_zero_rather_than_an_error(self):
        self.assertEqual(summarize([]), {"reached": 0, "of": 0, "first_failure": None})


class Skips(unittest.TestCase):
    def test_a_skip_names_the_step_it_waited_for(self):
        steps = []
        _skip(steps, "place", "open_map")
        self.assertEqual(steps[0]["status"], "skip")
        self.assertIn("open_map", steps[0]["detail"])

    def test_a_dependency_that_skipped_is_not_a_pass(self):
        # Otherwise a skip would satisfy its dependents, and one early failure would leave the rest
        # of the report claiming they passed.
        steps = [step("open_map", "skip")]
        self.assertFalse(steps_ok(steps, "open_map"))

    def test_a_missing_dependency_is_not_a_pass(self):
        self.assertFalse(steps_ok([], "open_map"))


if __name__ == "__main__":
    unittest.main()


class TheImportStep(unittest.TestCase):
    """**`--glb` adds an eleventh step and moves `of`.**

    The count is the loop's headline denominator, so a run with a mesh and a run without must be
    told apart rather than both reporting "of 10".
    """

    def test_ten_steps_without_a_mesh_eleven_with_one(self):
        without = [step(i, "pass") for i in ("tools", "make_map", "save")]
        with_import = without + [step("import", "pass")]
        self.assertEqual(summarize(without)["of"], 3)
        self.assertEqual(summarize(with_import)["of"], 4)
        self.assertEqual(summarize(with_import)["reached"], 4)

    def test_a_failed_import_is_the_first_failure(self):
        steps = [step("tools", "pass"), step("import", "fail")]
        self.assertEqual(summarize(steps)["first_failure"], "import")
