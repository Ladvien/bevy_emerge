"""check.py's exit codes against a stand-in endpoint (never a live model): a calibrated blank-frame check
fails a flat capture and passes a good one; an uncalibrated check never fails the run; a down endpoint is exit 2."""
import json, os, struct, subprocess, sys, tempfile, threading, unittest, zlib, base64
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

HERE = Path(__file__).resolve().parent


def png(pixels, w=8, h=8):
    raw = b"".join(b"\x00" + bytes(c for px in pixels[y * w:(y + 1) * w] for c in px) for y in range(h))
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


def is_flat(data_url):
    raw = zlib.decompress(_idat(base64.b64decode(data_url.split(",", 1)[1])))
    pixels = {raw[i * 25 + 1 + x * 3:i * 25 + 4 + x * 3] for i in range(8) for x in range(8)}
    return len(pixels) == 1


def _idat(b):
    i = 8
    while i < len(b):
        n = struct.unpack(">I", b[i:i + 4])[0]
        if b[i + 4:i + 8] == b"IDAT":
            return b[i + 8:i + 8 + n]
        i += 12 + n


class Stub(BaseHTTPRequestHandler):
    def do_POST(self):
        req = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        p = 0.97 if is_flat(req["images"][0]) else 0.02
        body = json.dumps({"answers": {q: {"type": "noul", "noul": p} for q in req["questions"]}}).encode()
        self.send_response(200); self.send_header("Content-Length", str(len(body))); self.end_headers(); self.wfile.write(body)

    def log_message(self, *a):
        pass


def checks_ron(calibration):
    cal = 'Some((check: "blank_frame", model: "stub", set_size: 20, agreement: 0.95, threshold: 0.7, date: "2026-10-29"))' if calibration else "None"
    return f'(checks: [(name: "blank_frame", question: "Is the image blank?", images: ["view"], facts: false, threshold: 0.7, after_edit: false, calibration: {cal})])'


class CheckPy(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.srv = HTTPServer(("127.0.0.1", 0), Stub)
        threading.Thread(target=cls.srv.serve_forever, daemon=True).start()
        cls.env = {**os.environ, "EMERGE_DECIDE_ENDPOINT": f"http://127.0.0.1:{cls.srv.server_port}/v1/systemone", "EMERGE_DECIDE_TIMEOUT_SECS": "5"}

    @classmethod
    def tearDownClass(cls):
        cls.srv.shutdown()

    def run_check(self, image, calibrated, env=None):
        with tempfile.TemporaryDirectory() as d:
            ron = Path(d) / "checks.ron"
            ron.write_text(checks_ron(calibrated))
            img = Path(d) / "view.png"
            img.write_bytes(image)
            cmd = ["cargo", "run", "-q", "-p", "bevy_emerge_decide", "--example", "check", "--", "run",
                   "--checks", str(ron), "--image", f"view={img}"]
            return subprocess.run(cmd, cwd=HERE.parents[1], env=env or self.env, capture_output=True, text=True).returncode

    flat = png([(200, 0, 200)] * 64)
    good = png([(x * 30 % 256, y * 30 % 256, (x + y) * 15) for y in range(8) for x in range(8)])

    def test_calibrated_blank_check_fails_flat_and_passes_good(self):
        self.assertEqual(self.run_check(self.flat, True), 1)
        self.assertEqual(self.run_check(self.good, True), 0)

    def test_uncalibrated_check_never_fails_the_run(self):
        self.assertEqual(self.run_check(self.flat, False), 0)

    def test_a_down_endpoint_is_not_a_pass(self):
        self.assertEqual(self.run_check(self.flat, True, {**self.env, "EMERGE_DECIDE_ENDPOINT": "http://127.0.0.1:1/x"}), 2)


if __name__ == "__main__":
    unittest.main()
