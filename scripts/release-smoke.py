#!/usr/bin/env python3
"""Verify a packaged desktop executable stays alive through startup."""

import subprocess
import sys
import time


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: release-smoke.py EXECUTABLE", file=sys.stderr)
        return 2

    process = subprocess.Popen([sys.argv[1]])
    try:
        time.sleep(10)
        status = process.poll()
        if status is not None:
            print(f"application exited during startup smoke test ({status})", file=sys.stderr)
            return 1
        print("application remained running through startup smoke test")
        return 0
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == "__main__":
    raise SystemExit(main())
