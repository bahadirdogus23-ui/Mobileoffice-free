#!/usr/bin/env bash
# Read engine-produced documents using independent OOXML libraries.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
export MO_INTEROP_DIR="$scratch"
cd "$root/rust"
cargo test --locked -p mo-word tests::exports_pdf_and_other_word_formats -- --exact
cargo test --locked -p mo-word tests::reads_uyap_udf_documents -- --exact
cargo test --locked -p mo-grid tests::blank_workbook_edits_through_commands_and_round_trips_as_xlsx -- --exact
cargo test --locked -p mo-deck tests::edits_text_and_slides_and_saves_pptx -- --exact
python3 - "$scratch" <<'PY'
import pathlib
import sys
from docx import Document
from openpyxl import load_workbook
from pptx import Presentation

root = pathlib.Path(sys.argv[1])
doc = Document(root / "word.docx")
text = "\n".join(p.text for p in doc.paragraphs)
assert "Başlık" in text and "İçerik ğüşıöç." in text, text
udf = Document(root / "udf.docx")
udf_text = "\n".join(p.text for p in udf.paragraphs)
for expected in ("İSTANBUL 1. ASLİYE HUKUK MAHKEMESİ", "Dava: Alacak", "Karar No: 2026/15"):
    assert expected in udf_text, udf_text
assert udf.tables, "UDF table missing after DOCX conversion"
cells = [cell.text for table in udf.tables for row in table.rows for cell in row.cells]
assert "Ad" in cells and "Soyad" in cells, cells
book = load_workbook(root / "grid.xlsx", data_only=False)
sheet = book.worksheets[0]
assert sheet["A1"].value == "Gelir"
assert sheet["B1"].value == 1200
assert sheet["B2"].value == "=B1*2"
cached = load_workbook(root / "grid.xlsx", data_only=True)
assert cached.worksheets[0]["B2"].value == 2400
deck = Presentation(root / "deck.pptx")
assert len(deck.slides) >= 2
for slide in list(deck.slides)[:2]:
    assert any("İğdır'da güneşli" in shape.text for shape in slide.shapes if shape.has_text_frame)
print("Independent OOXML readers: DOCX Turkish text, UDF text/table, XLSX formula/value, PPTX edited/copied text passed.")
PY

# Lossless-save corpus (docs/KAYIPSIZ-KAYIT-KORPUSU.md): generate richer files with the
# independent libraries, open → edit → save them through the bridges, compare what comes back.
corpus="$scratch/corpus"
python3 "$root/scripts/office_corpus.py" generate "$corpus"
export MO_CORPUS_DIR="$corpus"
cargo test --locked -p mo-word tests::corpus_round_trip -- --exact
cargo test --locked -p mo-grid tests::corpus_round_trip -- --exact
cargo test --locked -p mo-deck tests::corpus_round_trip -- --exact
python3 "$root/scripts/office_corpus.py" check "$corpus"
