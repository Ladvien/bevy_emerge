"""Timing regressions for record.py. Run: python3 -m unittest test_record (from tools/demo)."""
import time
import unittest

import record


class SlowAgent:
    """Stands in for McpClient: an MCP call that takes a while to answer."""

    def __init__(self, seconds):
        self.seconds = seconds

    def json_call(self, tool, args):
        time.sleep(self.seconds)
        return {"ok": True}


class McpEventIsStampedWhenCalled(unittest.TestCase):
    def test_the_event_is_stamped_at_call_start_not_return(self):
        tl = record.Timeline()
        time.sleep(0.05)
        before = tl.now()
        step = {"_kind": "mcp", "mcp": "emerge_frame_view", "args": {}, "at": 0.0}
        record.do_step(step, tl, None, SlowAgent(0.5))
        (event,) = tl.events
        # The call took 0.5 s. The event must be stamped when it started, not when it returned.
        self.assertLess(event["t"] - before, 0.25, f"stamped {event['t'] - before:.3f} s after the call began")


if __name__ == "__main__":
    unittest.main()
