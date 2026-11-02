#!/usr/bin/env python3
"""Exercise FTXUI in a controlling PTY and its local UDP event protocol."""

import fcntl
import os
import pty
import select
import signal
import socket
import struct
import sys
import termios
import time


def read_terminal(master: int) -> bytes:
    os.set_blocking(master, False)
    if not select.select([master], [], [], 0)[0]:
        return b""
    try:
        return os.read(master, 65536)
    except (BlockingIOError, OSError):
        return b""


def wait_for(pid: int, master: int, output: bytearray, seconds: float) -> int | None:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        output.extend(read_terminal(master))
        waited, status = os.waitpid(pid, os.WNOHANG)
        if waited == pid:
            return os.waitstatus_to_exitcode(status)
        time.sleep(0.02)
    output.extend(read_terminal(master))
    return None


def run_dashboard(binary: str, stop_with_ctrl_c: bool) -> tuple[int, bytes]:
    parent = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    parent.bind(("127.0.0.1", 0))
    parent.settimeout(5)
    pid, master = pty.fork()
    if pid == 0:
        os.execv(
            binary,
            [binary, str(parent.getsockname()[1]), str(os.getppid())],
        )

    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 32, 120, 0, 0))
    try:
        ready, _ = parent.recvfrom(128)
        if not ready.startswith(b"READY\t"):
            raise RuntimeError(f"unexpected helper handshake: {ready!r}")
        port = int(ready.split(b"\t", 1)[1])
        target = ("127.0.0.1", port)
        parent.sendto(b"INFO\tFTXUI\tKamafeu console smoke-test", target)
        parent.sendto(b"WARN\tFTXUI\twarning transport", target)
        time.sleep(0.25)
        if stop_with_ctrl_c:
            os.write(master, b"\x03")
        else:
            parent.sendto(b"STOP", target)
        output = bytearray()
        status = wait_for(pid, master, output, 5)
        if status is None:
            os.kill(pid, signal.SIGKILL)
            os.waitpid(pid, 0)
            raise RuntimeError("FTXUI helper did not exit after STOP; " + repr(output[-2000:]))
        if status != 0:
            raise RuntimeError(f"FTXUI helper exited with status {status}")
        if b"smoke-test" not in output:
            raise RuntimeError("FTXUI screen did not render the transported event: " + repr(output[-2000:]))
        if b"\x1b[?1049l" not in output:
            raise RuntimeError("FTXUI did not restore the primary terminal screen: " + repr(output[-2000:]))
        return status, bytes(output)
    finally:
        parent.close()
        os.close(master)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: smoke.py KAMAFEU_TUI", file=sys.stderr)
        return 2

    run_dashboard(sys.argv[1], stop_with_ctrl_c=False)
    run_dashboard(sys.argv[1], stop_with_ctrl_c=True)
    print("FTXUI PTY, event transport, Ctrl+C, STOP, and terminal restoration passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
