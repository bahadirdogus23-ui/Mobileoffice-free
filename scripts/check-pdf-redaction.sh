#!/usr/bin/env bash
# Verify persisted redaction with Poppler, independently of the Rust PDF reader.
set -euo pipefail
command -v pdftotext >/dev/null || { echo 'Install poppler-utils (pdftotext) first.' >&2; exit 1; }
root="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
cd "$root/rust"
DUMP="$scratch/redacted.pdf" cargo test --locked -p mo-pdf tests::blacks_out_personal_data_for_good -- --exact
test -s "$scratch/redacted.pdf"
pdftotext -layout "$scratch/redacted.pdf" "$scratch/redacted.txt"
python3 - "$scratch/redacted.txt" <<'PY'
import pathlib
import sys

text = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
for secret in ("10000000146", "TR33", "0532", "sule@ornek", "Şule"):
    if secret in text:
        raise SystemExit(f"Redacted text remains extractable: {secret}")
for retained in ("Yılmaz", "İSTANBUL"):
    if retained not in text:
        raise SystemExit(f"Expected non-redacted text missing: {retained}")
print("Saved PDF: sensitive text absent and non-redacted text preserved (Poppler).")
PY
MO_REDACTION_CORPUS_DIR="$scratch" cargo test --locked -p mo-pdf tests::redacts_all_58_ids_after_reopening_marks -- --exact
pdftotext -layout "$scratch/before.pdf" "$scratch/before.txt"
pdftotext -layout "$scratch/after.pdf" "$scratch/after.txt"
python3 - "$scratch" <<'PY'
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
before = (root / "before.txt").read_text(encoding="utf-8")
after = (root / "after.txt").read_text(encoding="utf-8")
ids = (root / "ids.txt").read_text().splitlines()
if len(ids) != 58 or len(set(ids)) != 58:
    raise SystemExit("Expected 58 distinct synthetic identifiers")
compact_before = "".join(before.split())
for identity in ids:
    if compact_before.count(identity) != 1:
        raise SystemExit("Original PDF does not expose every expected identifier exactly once")
if any(c.isascii() and c.isdigit() for c in after):
    raise SystemExit("Redacted PDF still exposes digits")
if before.count("KEEP THIS TEXT") != 3 or after.count("KEEP THIS TEXT") != 3:
    raise SystemExit("Non-redacted text was lost on one or more pages")
print("Three-page PDF: all 58 identifiers removed, all page markers preserved (Poppler).")
PY
