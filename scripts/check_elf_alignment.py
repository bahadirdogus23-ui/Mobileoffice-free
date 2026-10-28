#!/usr/bin/env python3
"""Fails when a 64-bit Android .so has a LOAD segment aligned below 16 KB.

Google Play requires 16 KB page-size support for apps with native code that target Android 15+.
32-bit ABIs (armeabi-v7a) are exempt: devices with 16 KB pages are 64-bit only.
Usage: check_elf_alignment.py <dir-or-.so> ...
"""
import pathlib
import struct
import sys

PT_LOAD = 1
MIN_ALIGN = 16 * 1024


def load_alignments(data: bytes):
    if data[:4] != b"\x7fELF":
        raise ValueError("not an ELF file")
    is64 = data[4] == 2
    endian = "<" if data[5] == 1 else ">"
    if is64:
        phoff, = struct.unpack_from(endian + "Q", data, 0x20)
        phentsize, phnum = struct.unpack_from(endian + "HH", data, 0x36)
    else:
        phoff, = struct.unpack_from(endian + "I", data, 0x1C)
        phentsize, phnum = struct.unpack_from(endian + "HH", data, 0x2A)
    aligns = []
    for i in range(phnum):
        off = phoff + i * phentsize
        p_type, = struct.unpack_from(endian + "I", data, off)
        if p_type != PT_LOAD:
            continue
        align, = struct.unpack_from(endian + ("Q" if is64 else "I"), data, off + (0x30 if is64 else 0x1C))
        aligns.append(align)
    return is64, aligns


def main(argv):
    files = []
    for arg in argv[1:] or ["."]:
        p = pathlib.Path(arg)
        files += sorted(p.rglob("*.so")) if p.is_dir() else [p]
    if not files:
        print("no .so files found", file=sys.stderr)
        return 1
    bad = 0
    for f in files:
        is64, aligns = load_alignments(f.read_bytes())
        low = min(aligns) if aligns else 0
        ok = (not is64) or low >= MIN_ALIGN
        print(f"{'OK  ' if ok else 'FAIL'} {f} (LOAD align {low // 1024} KB{'' if is64 else ', 32-bit: exempt'})")
        bad += not ok
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
