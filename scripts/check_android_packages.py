#!/usr/bin/env python3
"""Validate packaged native engines and report archive size without extracting files."""
import argparse
import json
import pathlib
import zipfile

from check_elf_alignment import MIN_ALIGN, load_alignments

ABIS = {"arm64-v8a", "armeabi-v7a", "x86_64"}


def inspect_package(path, expected_abis):
    path = pathlib.Path(path)
    if path.suffix not in {".apk", ".aab"}:
        raise ValueError(f"{path}: expected .apk or .aab")
    prefix = "base/lib/" if path.suffix == ".aab" else "lib/"
    with zipfile.ZipFile(path) as archive:
        entries = archive.infolist()
        names = [item.filename for item in entries]
        if len(names) != len(set(names)):
            raise ValueError(f"{path}: duplicate ZIP entries")
        required = "BundleConfig.pb" if path.suffix == ".aab" else "AndroidManifest.xml"
        if required not in names:
            raise ValueError(f"{path}: missing {required}")
        libraries = [item for item in entries if item.filename.startswith(prefix) and item.filename.endswith(".so")]
        present = {item.filename.split("/")[-2] for item in libraries}
        if present != set(expected_abis):
            raise ValueError(f"{path}: expected ABIs {sorted(expected_abis)}, found {sorted(present)}")
        for abi in expected_abis:
            if f"{prefix}{abi}/libmo_ffi.so" not in names:
                raise ValueError(f"{path}: missing native document engine for {abi}")
        native = []
        for item in libraries:
            data = archive.read(item)
            if len(data) < 20 or data[:4] != b"\x7fELF" or data[4] not in (1, 2) or data[5] not in (1, 2):
                raise ValueError(f"{path}: invalid ELF header for {item.filename}")
            is64, alignments = load_alignments(data)
            abi = item.filename.split("/")[-2]
            if is64 != (abi != "armeabi-v7a"):
                raise ValueError(f"{path}: wrong ELF class for {item.filename}")
            machine = int.from_bytes(data[18:20], "little" if data[5] == 1 else "big")
            if machine != {"arm64-v8a": 183, "armeabi-v7a": 40, "x86_64": 62}[abi]:
                raise ValueError(f"{path}: wrong ELF architecture for {item.filename}")
            if not alignments or (is64 and min(alignments) < MIN_ALIGN):
                raise ValueError(f"{path}: invalid LOAD alignment for {item.filename}")
            native.append({"path": item.filename, "bytes": item.file_size, "compressed_bytes": item.compress_size})
        return {
            "package": str(path),
            "bytes": path.stat().st_size,
            "abis": sorted(present),
            "arm64_apk_target_bytes": 60_000_000 if path.suffix == ".apk" and set(expected_abis) == {"arm64-v8a"} else None,
            "native_libraries": native,
            "largest_entries": [
                {"path": item.filename, "bytes": item.file_size, "compressed_bytes": item.compress_size}
                for item in sorted(entries, key=lambda item: item.compress_size, reverse=True)[:20]
            ],
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=pathlib.Path)
    parser.add_argument("--abis", nargs="+", choices=sorted(ABIS), required=True)
    parser.add_argument("--report", type=pathlib.Path, required=True)
    args = parser.parse_args()
    report = inspect_package(args.package, args.abis)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"OK {args.package}: {report['bytes'] / 1_000_000:.1f} MB; ABIs {', '.join(report['abis'])}")
    target = report["arm64_apk_target_bytes"]
    if target and report["bytes"] > target:
        print("Size target exceeded: ARM64 APK is larger than 60 MB; see the size report.")


if __name__ == "__main__":
    main()
