"""Compare the classic CLI against explicit ABI timing for the same PCM."""
import ctypes as c
import math
import pathlib
import struct
import subprocess
import sys
import tempfile
import wave

class Pitch(c.Structure):
    _fields_ = [("time_ms", c.c_double), ("cents", c.c_double), ("shape", c.c_uint32)]

class Options(c.Structure):
    _fields_ = [
        ("struct_size", c.c_uint32), ("sample_rate", c.c_uint32),
        ("input", c.POINTER(c.c_float)), ("input_length", c.c_uint64),
        *[(name, c.c_double) for name in ("duration_ms", "offset_ms", "consonant_ms", "cutoff_ms", "pitch_hz")],
        ("pitch_points", c.POINTER(Pitch)), ("pitch_point_count", c.c_uint32),
        *[(name, c.c_double) for name in ("gender", "pitch_cents", "breathiness", "high_frequency_cutoff", "frame_period_ms", "f0_floor", "f0_ceiling")],
        ("f0_method", c.c_uint32),
        *[(name, c.c_double) for name in ("loop_start_ms", "loop_end_ms", "tail_start_ms")],
        ("cancel", c.c_void_p), ("cancel_user_data", c.c_void_p),
        *[(name, c.c_double) for name in ("formant_shift", "consonant_adjustment", "target_consonant_ms")],
        ("has_target_consonant", c.c_uint32),
    ]

class Pcm(c.Structure):
    _fields_ = [("struct_size", c.c_uint32), ("samples", c.POINTER(c.c_float)),
                ("length", c.c_uint64), ("sample_rate", c.c_uint32)]

class Error(c.Structure):
    _fields_ = [("struct_size", c.c_uint32), ("code", c.c_int32), ("message", c.c_char * 256)]


def main():
    executable, library = map(pathlib.Path, sys.argv[1:])
    lib = c.CDLL(str(library.resolve()))
    lib.lt_render.argtypes = [c.POINTER(Options), c.POINTER(Pcm), c.POINTER(Error)]
    lib.lt_pcm_free.argtypes = [c.POINTER(Pcm)]
    assert lib.lt_render_options_size() == c.sizeof(Options)
    with tempfile.TemporaryDirectory(prefix="tainah-cli-timing-") as directory:
        source = pathlib.Path(directory) / "source.wav"
        output = pathlib.Path(directory) / "output.wav"
        rate = 44100
        pcm = []
        noise_state = 17
        for i in range(rate // 2):
            noise_state = (noise_state * 1664525 + 1013904223) & 0xffffffff
            phase = 2 * math.pi * 220 * i / rate
            value = sum(math.sin(n * phase) / n for n in range(1, 31))
            pcm.append(round(4500 * (0.3 + i / rate) * value + (noise_state / 0xffffffff - 0.5) * 100))
        with wave.open(str(source), "wb") as wav:
            wav.setparams((1, 2, rate, 0, "NONE", "not compressed"))
            wav.writeframes(struct.pack("<" + "h" * len(pcm), *pcm))
        data = (c.c_float * len(pcm))(*(value / 32768 for value in pcm))
        alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        values = [0] * 10 + [100] * 10 + [-100] * 10
        curve = "AA#9#Bk#9#" + alphabet[((-100) & 4095) >> 6] + alphabet[(-100) & 63] + "#9#"
        comparisons = 0
        for tempo in (60, 120, 138, 180):
            points = (Pitch * len(values))(*(Pitch(i * 60000 * 5 / (tempo * 480), value, 1)
                                                   for i, value in enumerate(values)))
            for velocity in (50, 100, 150, 200):
                o = Options()
                o.struct_size = c.sizeof(o); o.sample_rate = rate
                o.input = data; o.input_length = len(data)
                o.duration_ms = 240; o.offset_ms = 70; o.consonant_ms = 90; o.cutoff_ms = -350
                o.pitch_hz = 220; o.pitch_points = points; o.pitch_point_count = len(points)
                o.frame_period_ms = 5; o.f0_floor = 55; o.f0_ceiling = 1760; o.f0_method = 1
                o.consonant_adjustment = 25
                o.target_consonant_ms = 90 * 2 ** (1 - velocity / 100); o.has_target_consonant = 1
                result = Pcm(); result.struct_size = c.sizeof(result)
                error = Error(); error.struct_size = c.sizeof(error)
                status = lib.lt_render(c.byref(o), c.byref(result), c.byref(error))
                assert status == 0, error.message.decode()
                try:
                    expected = [round(max(-1, min(1, result.samples[i])) * 32767) for i in range(result.length)]
                finally:
                    lib.lt_pcm_free(c.byref(result))
                arguments = [str(executable), str(source), str(output), "A3", str(velocity), "C25",
                             "70", "240", "90", "-350", "100", "0", f"!{tempo}", curve]
                subprocess.run(arguments, check=True, stdout=subprocess.DEVNULL)
                with wave.open(str(output), "rb") as wav:
                    assert wav.getframerate() == rate and wav.getnchannels() == 1
                    actual = [value[0] for value in struct.iter_unpack("<h", wav.readframes(wav.getnframes()))]
                assert len(actual) == len(expected) == 10584
                delta = max(abs(a - b) for a, b in zip(actual, expected))
                assert delta <= 1, (tempo, velocity, delta)
                comparisons += 1
        for invalid in ("!0", "!nan", "!120garbage"):
            arguments[-2] = invalid
            bad = subprocess.run(arguments, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            assert bad.returncode == 2, (invalid, bad.returncode)
        print(f"CLI timing: {comparisons} ABI comparisons passed; invalid tempos rejected.")

if __name__ == "__main__":
    main()
