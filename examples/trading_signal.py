#!/usr/bin/env python3
"""Create and optionally play a short trading-signal announcement.

Build the shared library first with ``cargo build --release``.
"""

from __future__ import annotations

import argparse
import ctypes
import os
import shutil
import subprocess
import sys
from pathlib import Path


# 39 words: short enough for a signal-group announcement.
DEFAULT_SIGNAL = (
    "Bitcoin against U S D T is showing bullish momentum on the fifteen-minute chart. "
    "Consider a long entry near support, with a stop below the recent low and targets "
    "at resistance. Manage risk carefully; this is not financial advice."
)

# The enum value is generated from src/voices.rs. It is also documented in README.md.
VOICE_EN_GB_THOMAS_NEURAL = 110
READ_ALOUD_STATUS_SUCCESS = 0


class ReadAloudSpeechOptions(ctypes.Structure):
    _fields_ = [
        ("size", ctypes.c_uint32),
        ("pitch_hz", ctypes.c_int32),
        ("rate", ctypes.c_float),
        ("volume", ctypes.c_float),
    ]


def library_names() -> tuple[str, ...]:
    if sys.platform == "win32":
        return ("read_aloud.dll",)
    if sys.platform == "darwin":
        return ("libread_aloud.dylib",)
    return ("libread_aloud.so",)


def load_library() -> ctypes.CDLL:
    configured_path = os.environ.get("READ_ALOUD_LIBRARY")
    project_root = Path(__file__).resolve().parents[1]
    candidates = []

    if configured_path:
        candidates.append(Path(configured_path).expanduser())

    for build_type in ("release", "debug"):
        candidates.extend(
            project_root.joinpath("target", build_type, name)
            for name in library_names()
        )

    for candidate in candidates:
        if candidate.is_file():
            return ctypes.CDLL(str(candidate))

    searched = "\n  ".join(str(path) for path in candidates)
    raise FileNotFoundError(
        "Could not find the read_aloud shared library. "
        "Run 'cargo build --release' or set READ_ALOUD_LIBRARY.\n"
        f"Searched:\n  {searched}"
    )


def configure_library(lib: ctypes.CDLL) -> None:
    lib.read_aloud_text_to_speech.argtypes = [
        ctypes.c_char_p,
        ctypes.c_int,
        ctypes.POINTER(ReadAloudSpeechOptions),
        ctypes.c_char_p,
    ]
    lib.read_aloud_text_to_speech.restype = ctypes.c_int
    lib.read_aloud_status_string.argtypes = [ctypes.c_int]
    lib.read_aloud_status_string.restype = ctypes.c_char_p
    lib.read_aloud_last_error_message.argtypes = []
    lib.read_aloud_last_error_message.restype = ctypes.c_char_p


def create_audio(text: str, output_path: Path) -> None:
    lib = load_library()
    configure_library(lib)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    status = lib.read_aloud_text_to_speech(
        text.encode("utf-8"),
        VOICE_EN_GB_THOMAS_NEURAL,
        None,
        os.fsencode(output_path),
    )
    if status != READ_ALOUD_STATUS_SUCCESS:
        status_name = lib.read_aloud_status_string(status).decode("utf-8")
        details = lib.read_aloud_last_error_message().decode("utf-8")
        raise RuntimeError(f"read_aloud failed ({status_name}): {details}")


def play_audio(output_path: Path) -> None:
    if sys.platform == "darwin":
        players = [("afplay", ["afplay", str(output_path)])]
    elif sys.platform == "win32":
        os.startfile(os.path.abspath(output_path))  # type: ignore[attr-defined]
        return
    else:
        players = [
            (
                "ffplay",
                ["ffplay", "-nodisp", "-autoexit", "-loglevel", "quiet", str(output_path)],
            ),
            ("mpv", ["mpv", "--no-video", str(output_path)]),
            ("mpg123", ["mpg123", "-q", str(output_path)]),
        ]

    for command, args in players:
        if shutil.which(command):
            subprocess.run(args, check=True)
            return

    raise RuntimeError(
        "Audio was created, but no player was found. Install ffplay, mpv, or mpg123, "
        "then run the script again with --play."
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--text",
        default=DEFAULT_SIGNAL,
        help="announcement text; it must contain 30 to 40 words",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("demo.mp3"),
        help="output MP3 path (default: %(default)s)",
    )
    parser.add_argument(
        "--play",
        action="store_true",
        help="play the generated MP3 after creating it",
    )
    args = parser.parse_args()
    word_count = len(args.text.split())
    if not 30 <= word_count <= 40:
        parser.error(f"--text must contain 30 to 40 words (got {word_count})")
    return args


def main() -> int:
    args = parse_args()
    create_audio(args.text, args.output)
    print(f"Created {args.output} ({len(args.text.split())} words).")
    if args.play:
        play_audio(args.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (FileNotFoundError, RuntimeError, OSError, subprocess.CalledProcessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        raise SystemExit(1)
