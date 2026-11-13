#!/usr/bin/env python3
"""Lossless-save corpus: open → edit → save → independent reader.

`generate DIR` writes a small synthetic DOCX/XLSX/PPTX corpus with python-docx, openpyxl and
python-pptx (nothing is downloaded and nothing binary is committed). The Rust tests
`tests::corpus_round_trip` of mo-word, mo-grid and mo-deck then open each file with the app's
bridge, make one edit through the engine and write `<name>.saved.<ext>`. `check DIR` reads the
saved files back with the same independent libraries and compares them with what was generated.

Everything generated must still be there, plus the edit. What the engines are known to lose today
is listed in KNOWN_LOSSES (and in docs/KAYIPSIZ-KAYIT-KORPUSU.md): for those items the exact
current behaviour is asserted, so a fix and a new regression are both noticed.
"""

import datetime
import pathlib
import struct
import sys
import zipfile
import zlib

from docx import Document
from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_BREAK
from docx.shared import Pt, RGBColor
from openpyxl import Workbook, load_workbook
from openpyxl.comments import Comment
from openpyxl.styles import Alignment, Border, Font, PatternFill, Side
from openpyxl.workbook.defined_name import DefinedName
from openpyxl.worksheet.datavalidation import DataValidation
from lxml import etree
from pptx import Presentation
from pptx.dml.color import RGBColor as PptxRGB
from pptx.util import Inches
from pptx.util import Pt as PptxPt

TURKISH = "İstanbul'da ılık bir gün: Şişli, Ağrı, Üsküdar, Ödemiş, Çeşme — ığüşöç İĞÜŞÖÇ."
EDIT_WORD = "Düzenlendi "
EDIT_DECK = " (düzenlendi)"
EDIT_GRID = ("Veri", "B3", 75)  # sheet, cell, new value: feeds the SUM/IF/VLOOKUP formulas

def png(width=4, height=3, rgb=(200, 30, 60)):
    """A tiny opaque PNG, written by hand (no imaging library)."""

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    rows = b"".join(b"\x00" + bytes(rgb) * width for _ in range(height))
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


# --------------------------------------------------------------------------------------------
# DOCX


def make_docx(path, picture):
    doc = Document()
    doc.add_heading("Kayıpsız Kayıt Raporu", level=1)
    doc.add_heading("Giriş ve Amaç", level=2)
    p = doc.add_paragraph("İlk paragraf: ")
    p.add_run("kalın").bold = True
    p.add_run(", ")
    p.add_run("italik").italic = True
    p.add_run(", ")
    p.add_run("altı çizili").underline = True
    p.add_run(" ve ")
    red = p.add_run("kırmızı")
    red.font.color.rgb = RGBColor(0xC0, 0x10, 0x20)
    p.add_run(" sözcükler, ")
    big = p.add_run("büyük Arial")
    big.font.size = Pt(16)
    big.font.name = "Arial"
    p.add_run(".")
    centred = doc.add_paragraph(TURKISH)
    centred.alignment = WD_ALIGN_PARAGRAPH.CENTER
    for item in ("Birinci madde: elma", "İkinci madde: armut", "Üçüncü madde: ıhlamur"):
        doc.add_paragraph(item, style="List Bullet")
    for item in ("Adım bir", "Adım iki", "Adım üç"):
        doc.add_paragraph(item, style="List Number")
    table = doc.add_table(rows=3, cols=3)
    table.style = "Table Grid"
    for r in range(3):
        for c in range(3):
            table.cell(r, c).text = f"H{r + 1}{c + 1}"
    merged = table.cell(0, 0).merge(table.cell(0, 1))
    merged.text = "Birleşik başlık"
    table.cell(1, 2).merge(table.cell(2, 2)).text = "Dikey birleşik"
    doc.add_paragraph("Resim aşağıda:")
    doc.add_paragraph().add_run().add_picture(str(picture), width=Pt(40))
    doc.add_paragraph("Sayfa sonu öncesi son satır.").add_run().add_break(WD_BREAK.PAGE)
    doc.add_paragraph("İkinci sayfa: Öğrenci Çağlar Işık.")
    section = doc.sections[0]
    section.left_margin = Pt(54)
    section.top_margin = Pt(64)
    doc.core_properties.title = "Kayıpsız kayıt örneği"
    doc.core_properties.author = "Gülşen Işık"
    doc.core_properties.subject = "Derlem"
    section.header.paragraphs[0].text = "Üst bilgi: Gizli ve Özel"
    section.footer.paragraphs[0].text = "Alt bilgi: Sayfa düzeni Ğ"
    doc.save(path)


def has_numbering(paragraph):
    """Whether the paragraph is a list item: numbering on the paragraph or on its style chain."""
    if paragraph._p.pPr is not None and paragraph._p.pPr.numPr is not None:
        return True
    style = paragraph.style
    while style is not None:
        ppr = style.element.pPr
        if ppr is not None and ppr.numPr is not None:
            return True
        style = style.base_style
    return False


def docx_facts(path):
    doc = Document(path)
    body = doc.element.body
    paras = [p.text for p in doc.paragraphs]
    runs = {}
    for p in doc.paragraphs:
        for r in p.runs:
            marks = set()
            if r.bold:
                marks.add("b")
            if r.italic:
                marks.add("i")
            if r.underline:
                marks.add("u")
            if r.font.color is not None and r.font.color.type is not None and r.font.color.rgb is not None:
                marks.add(str(r.font.color.rgb))
            if r.font.size is not None:
                marks.add(f"{r.font.size.pt:g}pt")
            if r.font.name:
                marks.add(r.font.name)
            # Only directly formatted runs: plain runs may be split or joined freely.
            if marks and r.text.strip():
                runs.setdefault(r.text.strip(), set()).update(marks)
    styles = {p.text: p.style.name for p in doc.paragraphs if p.text}
    aligned = {p.text: str(p.alignment) for p in doc.paragraphs if p.text and p.alignment is not None}
    sections = [
        tuple(round(v.pt) if v is not None else None for v in (s.page_width, s.page_height, s.left_margin, s.right_margin, s.top_margin, s.bottom_margin))
        for s in doc.sections
    ]
    core = doc.core_properties
    numbered = {p.text for p in doc.paragraphs if p.text and has_numbering(p)}
    tables = []
    for t in doc.tables:
        cells = []
        for row in t.rows:
            cells.append([c.text for c in row.cells])
        spans = sorted(
            (int(tc.get("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}val")))
            for tc in t._tbl.iter("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}gridSpan")
        )
        vmerges = len(list(t._tbl.iter("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}vMerge")))
        tables.append({"cells": cells, "gridSpans": spans, "vMerges": vmerges})
    W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
    page_breaks = sum(1 for br in body.iter(W + "br") if br.get(W + "type") == "page")
    page_breaks += sum(1 for _ in body.iter(W + "pageBreakBefore"))
    pictures = [(round(s.width.pt), round(s.height.pt)) for s in doc.inline_shapes]
    headers = [p.text for s in doc.sections for p in s.header.paragraphs if p.text]
    footers = [p.text for s in doc.sections for p in s.footer.paragraphs if p.text]
    with zipfile.ZipFile(path) as z:
        media = sorted(n for n in z.namelist() if n.startswith("word/media/"))
        media_bytes = [z.read(n) for n in media]
    return {
        "paragraphs": paras,
        "runs": runs,
        "styles": styles,
        "numbered": numbered,
        "tables": tables,
        "page_breaks": page_breaks,
        "pictures": pictures,
        "media": media_bytes,
        "headers": headers,
        "footers": footers,
        "aligned": aligned,
        "sections": sections,
        "core": (core.title, core.author, core.subject),
    }


def check_docx(orig, saved, problems):
    a, b = docx_facts(orig), docx_facts(saved)
    name = orig.name
    # The edit: a word typed at the start of the document.
    first = next((t for t in b["paragraphs"] if t), "")
    expect(first.startswith(EDIT_WORD), f"{name}: edit missing, first paragraph is {first!r}", problems)
    edited = [t[len(EDIT_WORD):] if i == 0 and t.startswith(EDIT_WORD) else t for i, t in enumerate(x for x in b["paragraphs"] if x)]
    want = [t for t in a["paragraphs"] if t]
    expect(edited == want, f"{name}: paragraph texts differ:\n  want {want}\n  got  {edited}", problems)
    for text, marks in a["runs"].items():
        got = b["runs"].get(text)
        expect(got is not None and marks <= got, f"{name}: run {text!r} formatting {sorted(marks)} → {sorted(got) if got is not None else 'missing'}", problems)
    for text, style in a["styles"].items():
        key = EDIT_WORD + text if text == want[0] else text
        expect(b["styles"].get(key) == style, f"{name}: paragraph {text!r} style {style!r} → {b['styles'].get(key)!r}", problems)
    for text in a["numbered"]:
        expect(text in b["numbered"], f"{name}: list item {text!r} lost its numbering", problems)
    expect(len(b["tables"]) == len(a["tables"]), f"{name}: tables {len(a['tables'])} → {len(b['tables'])}", problems)
    for ta, tb in zip(a["tables"], b["tables"]):
        expect(ta["cells"] == tb["cells"], f"{name}: table cells differ:\n  want {ta['cells']}\n  got  {tb['cells']}", problems)
        expect(ta["gridSpans"] == tb["gridSpans"], f"{name}: horizontal merges {ta['gridSpans']} → {tb['gridSpans']}", problems)
        expect(ta["vMerges"] == tb["vMerges"], f"{name}: vertical merges {ta['vMerges']} → {tb['vMerges']}", problems)
    expect(b["page_breaks"] >= a["page_breaks"], f"{name}: page breaks {a['page_breaks']} → {b['page_breaks']}", problems)
    expect(b["pictures"] == a["pictures"], f"{name}: inline pictures (size in pt) {a['pictures']} → {b['pictures']}", problems)
    expect(sorted(b["media"]) == sorted(a["media"]), f"{name}: picture bytes changed ({len(a['media'])} → {len(b['media'])} files)", problems)
    expect(b["headers"] == a["headers"], f"{name}: header {a['headers']} → {b['headers']}", problems)
    expect(b["footers"] == a["footers"], f"{name}: footer {a['footers']} → {b['footers']}", problems)
    for text, al in a["aligned"].items():
        expect(b["aligned"].get(text) == al, f"{name}: paragraph {text!r} alignment {al} → {b['aligned'].get(text)}", problems)
    expect(b["sections"] == a["sections"], f"{name}: page size/margins (pt) {a['sections']} → {b['sections']}", problems)
    expect(b["core"] == a["core"], f"{name}: title/author/subject {a['core']} → {b['core']}", problems)


# --------------------------------------------------------------------------------------------
# XLSX


def make_xlsx(path):
    wb = Workbook()
    data = wb.active
    data.title = "Veri"
    data.append(["Ürün", "Adet", "Fiyat", "Tarih", "Stokta"])
    rows = [
        ("Çay", 40, 12.5, datetime.date(2026, 1, 15), True),
        ("Şeker", 25, 30.25, datetime.date(2026, 2, 1), False),
        ("Işıklı kalem", 60, 7.0, datetime.date(2026, 3, 9), True),
        ("Ğ harfli ürün", 10, 99.99, datetime.date(2026, 4, 30), True),
    ]
    for row in rows:
        data.append(list(row))
    data["A7"] = "Toplam"
    data["B7"] = "=SUM(B2:B5)"
    data["C7"] = '=IF(B7>100,"çok","az")'
    data["D7"] = '=VLOOKUP("Şeker",A2:C5,3,FALSE)'
    for r in range(2, 6):
        data.cell(r, 3).number_format = "#,##0.00"
        data.cell(r, 4).number_format = "dd.mm.yyyy"
    data["A1"].font = Font(bold=True)
    data["B1"].fill = PatternFill("solid", fgColor="FFFF00")
    thin = Side(style="thin", color="000000")
    data["C1"].border = Border(left=thin, right=thin, top=thin, bottom=thin)
    data.column_dimensions["A"].width = 22
    data.column_dimensions["D"].width = 14
    data["A7"].font = Font(italic=True, color="1F4E79")
    data["E1"].alignment = Alignment(horizontal="center", wrap_text=True)
    data.row_dimensions[1].height = 30
    data.freeze_panes = "A2"
    data.auto_filter.ref = "A1:E5"
    data.page_setup.orientation = "landscape"
    wb.properties.creator = "Gülşen Işık"
    wb.properties.title = "Kayıpsız kayıt tablosu"
    data.merge_cells("A9:C9")
    data["A9"] = "Birleşik açıklama satırı"
    dv = DataValidation(type="list", formula1='"Evet,Hayır,Belki"', allow_blank=True)
    data.add_data_validation(dv)
    dv.add("E9")
    data["E9"] = "Evet"
    data["A2"].comment = Comment("Çay notu: İthal değil.", "Gülşen")
    wb.defined_names["FiyatAraligi"] = DefinedName("FiyatAraligi", attr_text="Veri!$C$2:$C$5")

    summary = wb.create_sheet("Özet Çizelge")
    summary["A1"] = "Toplam adet"
    summary["B1"] = "=Veri!B7"
    summary["A2"] = "İki katı"
    summary["B2"] = "=B1*2"
    summary["A3"] = "Yüzde"
    summary["B3"] = 0.375
    summary["B3"].number_format = "0.0%"
    wb.create_sheet("Boş ığüşİ")
    wb.save(path)


def xlsx_facts(path, data_only):
    wb = load_workbook(path, data_only=data_only)
    sheets = {}
    for ws in wb.worksheets:
        cells = {}
        for row in ws.iter_rows():
            for c in row:
                if c.value is not None:
                    cells[c.coordinate] = c.value
        sheets[ws.title] = {
            "cells": cells,
            "formats": {c.coordinate: c.number_format for row in ws.iter_rows() for c in row if c.value is not None},
            "merges": sorted(str(m) for m in ws.merged_cells.ranges),
            "freeze": ws.freeze_panes,
            "widths": {k: round(v.width, 1) for k, v in ws.column_dimensions.items() if v.customWidth},
            "validations": sorted((dv.type, dv.formula1, str(dv.sqref), bool(dv.allow_blank), bool(dv.showDropDown), bool(dv.showErrorMessage)) for dv in ws.data_validations.dataValidation),
            "comments": {c.coordinate: (c.comment.text, c.comment.author) for row in ws.iter_rows() for c in row if c.comment},
            "bold": {c.coordinate for row in ws.iter_rows() for c in row if c.font and c.font.b},
            "italic": {c.coordinate for row in ws.iter_rows() for c in row if c.font and c.font.i},
            "colors": {c.coordinate: c.font.color.rgb[-6:] for row in ws.iter_rows() for c in row if c.font and c.font.color is not None and isinstance(c.font.color.rgb, str) and c.font.color.rgb[-6:] != "000000"},
            "align": {c.coordinate: (c.alignment.horizontal, bool(c.alignment.wrap_text)) for row in ws.iter_rows() for c in row if c.alignment and (c.alignment.horizontal or c.alignment.wrap_text)},
            "fills": {c.coordinate: c.fill.fgColor.rgb[-6:] for row in ws.iter_rows() for c in row if c.fill and c.fill.fill_type == "solid"},
            "heights": {k: v.height for k, v in ws.row_dimensions.items() if v.customHeight},
            "filter": ws.auto_filter.ref,
            "orientation": ws.page_setup.orientation or "portrait",
            "borders": {c.coordinate for row in ws.iter_rows() for c in row if c.border and c.border.left and c.border.left.style},
        }
    names = {n: dn.attr_text for n, dn in wb.defined_names.items()}
    props = (wb.properties.creator, wb.properties.title)
    return {"order": wb.sheetnames, "sheets": sheets, "names": names, "props": props}


def norm(v):
    """A cell value with its kind: 25 and 25.0 are the same number, but True is not 1."""
    if isinstance(v, bool):
        return ("bool", v)
    if isinstance(v, datetime.datetime) and v.time() == datetime.time(0):
        return ("date", v.date())
    if isinstance(v, (int, float)):
        return ("number", float(v))
    return (type(v).__name__, v)


def check_xlsx(orig, saved, problems):
    a, b = xlsx_facts(orig, False), xlsx_facts(saved, False)
    cached = xlsx_facts(saved, True)
    name = orig.name
    expect(b["order"] == a["order"], f"{name}: sheets {a['order']} → {b['order']}", problems)
    edit_sheet, edit_cell, edit_value = EDIT_GRID
    for title, sa in a["sheets"].items():
        sb = b["sheets"].get(title)
        if sb is None:
            continue
        for coord, value in sa["cells"].items():
            want = edit_value if (title, coord) == (edit_sheet, edit_cell) else value
            got = sb["cells"].get(coord)
            expect(norm(got) == norm(want), f"{name}: {title}!{coord} {want!r} → {got!r}", problems)
        extra = set(sb["cells"]) - set(sa["cells"])
        expect(not extra, f"{name}: {title}: unexpected cells {sorted(extra)}", problems)
        for coord, fmt in sa["formats"].items():
            expect(sb["formats"].get(coord) == fmt, f"{name}: {title}!{coord} number format {fmt!r} → {sb['formats'].get(coord)!r}", problems)
        for key in ("merges", "freeze", "validations", "comments", "bold", "italic", "colors", "align", "fills", "borders", "heights", "filter", "orientation"):
            expect(sb[key] == sa[key], f"{name}: {title}: {key} {sa[key]!r} → {sb[key]!r}", problems)
        for col, width in sa["widths"].items():
            got = sb["widths"].get(col)
            expect(got is not None and abs(got - width) < 0.6, f"{name}: {title}: column {col} width {width} → {got}", problems)
    expect(b["names"] == a["names"], f"{name}: defined names {a['names']} → {b['names']}", problems)
    expect(b["props"] == a["props"], f"{name}: creator/title {a['props']} → {b['props']}", problems)
    # Formulas: openpyxl writes none of their values, so the engine computed every cached value
    # (with the edit applied).
    data = cached["sheets"].get("Veri", {}).get("cells", {})
    summary = cached["sheets"].get("Özet Çizelge", {}).get("cells", {})
    total = 40 + edit_value + 60 + 10
    for where, got, want in (
        ("Veri!B7", data.get("B7"), total),
        ("Veri!C7", data.get("C7"), "çok" if total > 100 else "az"),
        ("Veri!D7", data.get("D7"), 30.25),
        ("Özet Çizelge!B1", summary.get("B1"), total),
        ("Özet Çizelge!B2", summary.get("B2"), total * 2),
    ):
        expect(norm(got) == norm(want), f"{name}: cached value of {where} {want!r} → {got!r}", problems)


# --------------------------------------------------------------------------------------------
# PPTX


def make_pptx(path, picture):
    prs = Presentation()
    s1 = prs.slides.add_slide(prs.slide_layouts[0])
    s1.shapes.title.text = "Çeyrek Sonuçları"
    s1.placeholders[1].text = "Hazırlayan: Ayşe Yılmaz — İzmir"
    s1.notes_slide.notes_text_frame.text = "Konuşmacı notu: Açılışta teşekkür et. ığüşöç"

    s2 = prs.slides.add_slide(prs.slide_layouts[1])
    s2.shapes.title.text = "Gündem"
    body = s2.placeholders[1].text_frame
    body.text = "Birinci başlık: Gelirler"
    for line in ("İkinci başlık: Giderler", "Üçüncü başlık: Şirket planı"):
        body.add_paragraph().text = line
    box = s2.shapes.add_textbox(Inches(1), Inches(6), Inches(6), Inches(0.6))
    para = box.text_frame.paragraphs[0]
    para.text = "Metin kutusu: "
    run = para.add_run()
    run.text = "Ölçüm Çalışması"
    run.font.bold = True
    run.font.size = PptxPt(24)
    run.font.color.rgb = PptxRGB(0x1F, 0x6F, 0x3A)
    s2.notes_slide.notes_text_frame.text = "İkinci slayt notu."

    s3 = prs.slides.add_slide(prs.slide_layouts[5])
    s3.shapes.title.text = "Tablo ve Resim"
    tbl = s3.shapes.add_table(3, 3, Inches(0.5), Inches(1.6), Inches(6), Inches(1.5)).table
    for r in range(3):
        for c in range(3):
            tbl.cell(r, c).text = f"Hücre {r + 1}.{c + 1} ğ"
    tbl.cell(2, 0).merge(tbl.cell(2, 1))
    s3.shapes.add_picture(str(picture), Inches(7), Inches(2), Inches(1), Inches(0.75))
    s3.notes_slide.notes_text_frame.text = "Tablo notu: Çarşamba günü güncellenecek."
    prs.core_properties.title = "Kayıpsız kayıt sunumu"
    prs.core_properties.author = "Gülşen Işık"
    prs.save(path)


def pptx_facts(path):
    prs = Presentation(path)
    slides = []
    for slide in prs.slides:
        texts, tables, pictures, placeholders, boxes, runs = [], [], [], [], [], {}
        for shape in slide.shapes:
            if shape.has_text_frame and shape.text_frame.text:
                texts.append(shape.text_frame.text)
                for para in shape.text_frame.paragraphs:
                    for r in para.runs:
                        marks = set()
                        if r.font.bold:
                            marks.add("b")
                        if r.font.size is not None:
                            marks.add(f"{r.font.size.pt:g}pt")
                        if r.font.color is not None and r.font.color.type is not None and getattr(r.font.color, "rgb", None) is not None:
                            marks.add(str(r.font.color.rgb))
                        if marks and r.text.strip():
                            runs.setdefault(r.text.strip(), set()).update(marks)
            if shape.is_placeholder:
                placeholders.append(shape.placeholder_format.type)
            else:
                # Shapes placed on the slide itself (placeholders may inherit their box).
                boxes.append(tuple(round(v / 12700) for v in (shape.left, shape.top, shape.width, shape.height)))
            if shape.has_table:
                cells = [[c.text for c in row.cells] for row in shape.table.rows]
                merges = sorted((r, c, cell.span_height, cell.span_width) for r, row in enumerate(shape.table.rows) for c, cell in enumerate(row.cells) if cell.is_merge_origin)
                tables.append({"cells": cells, "merges": merges})
            if shape.shape_type == 13:  # MSO_SHAPE_TYPE.PICTURE
                pictures.append(shape.image.blob)
        notes = slide.notes_slide.notes_text_frame.text if slide.has_notes_slide else ""
        slides.append(
            {
                "texts": texts,
                "runs": runs,
                "tables": tables,
                "pictures": pictures,
                "boxes": sorted(boxes),
                "notes": notes,
                "layout": slide.slide_layout.name,
                "placeholders": sorted(map(str, placeholders)),
            }
        )
    meta = (prs.slide_width, prs.slide_height, prs.core_properties.title, prs.core_properties.author)
    return slides, meta


def check_pptx(orig, saved, problems):
    (a, meta_a), (b, meta_b) = pptx_facts(orig), pptx_facts(saved)
    name = orig.name
    expect(meta_b == meta_a, f"{name}: slide size/title/author {meta_a} → {meta_b}", problems)
    expect(len(b) == len(a), f"{name}: slides {len(a)} → {len(b)}", problems)
    for i, (sa, sb) in enumerate(zip(a, b), start=1):
        want = list(sa["texts"])
        if i == 1:
            want[0] = want[0] + EDIT_DECK
        expect(sorted(sb["texts"]) == sorted(want), f"{name}: slide {i} texts:\n  want {want}\n  got  {sb['texts']}", problems)
        expect(sb["tables"] == sa["tables"], f"{name}: slide {i} tables {sa['tables']} → {sb['tables']}", problems)
        expect(sb["pictures"] == sa["pictures"], f"{name}: slide {i} pictures {len(sa['pictures'])} → {len(sb['pictures'])} (or their bytes changed)", problems)
        for text, marks in sa["runs"].items():
            got = sb["runs"].get(text)
            expect(got is not None and marks <= got, f"{name}: slide {i} run {text!r} formatting {sorted(marks)} → {sorted(got) if got is not None else 'missing'}", problems)
        expect(sb["boxes"] == sa["boxes"], f"{name}: slide {i} shape boxes (pt) {sa['boxes']} → {sb['boxes']}", problems)
        expect(sb["layout"] == sa["layout"], f"{name}: slide {i} layout {sa['layout']!r} → {sb['layout']!r}", problems)
        expect(sb["notes"] == sa["notes"], f"{name}: slide {i} notes {sa['notes']!r} → {sb['notes']!r}", problems)
        expect(sb["placeholders"] == sa["placeholders"], f"{name}: slide {i} placeholders {sa['placeholders']} → {sb['placeholders']}", problems)


# --------------------------------------------------------------------------------------------


def expect(ok, message, problems):
    if not ok:
        problems.append(message)


def content_types(path):
    """The kinds of part in an OOXML package: content type → number of parts."""
    with zipfile.ZipFile(path) as z:
        types = etree.fromstring(z.read("[Content_Types].xml"))
        names = [n for n in z.namelist() if not n.endswith("/") and n != "[Content_Types].xml"]
    ns = "{http://schemas.openxmlformats.org/package/2006/content-types}"
    defaults = {d.get("Extension").lower(): d.get("ContentType") for d in types.iter(ns + "Default")}
    overrides = {o.get("PartName").lstrip("/"): o.get("ContentType") for o in types.iter(ns + "Override")}
    kinds = {}
    for n in names:
        kind = overrides.get(n) or defaults.get(n.rsplit(".", 1)[-1].lower(), "?")
        kinds[kind] = kinds.get(kind, 0) + 1
    return kinds


def check_package(orig, saved, problems, known):
    """Every kind of part the original has is still there (parts may be renamed)."""
    a, b = content_types(orig), content_types(saved)
    allowed = {KNOWN_LOSSES[k]["part"] for k in known if "part" in KNOWN_LOSSES[k]}
    for kind, n in a.items():
        if kind not in allowed:
            expect(b.get(kind, 0) >= n, f"{orig.name}: parts of type {kind}: {n} → {b.get(kind, 0)}", problems)


def count(path, part, xpath):
    """How many XML nodes `xpath` (with the w/a namespaces) finds in `part` of the package."""
    with zipfile.ZipFile(path) as z:
        if part not in z.namelist():
            return 0
        root = etree.fromstring(z.read(part))
    return int(root.xpath(f"count({xpath})", namespaces=NS))


def parts(path):
    with zipfile.ZipFile(path) as z:
        return set(z.namelist())


NS = {
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
}

# Known losses: what the engines drop today, each with a probe that is true while the loss
# happens. docs/KAYIPSIZ-KAYIT-KORPUSU.md describes them in Turkish; keep the two in step. A probe
# that turns false means the loss was fixed (update the table); anything lost that is not listed
# here fails the check.
KNOWN_LOSSES = {
    "docx-tema-baglari": {
        "ext": "docx",
        "what": "styles' theme fonts and colours (asciiTheme, themeColor…) are written as fixed values",
        "where": "WordCraft DOCX writer (upstream)",
        "probe": lambda o, s: count(o, "word/styles.xml", "//@w:asciiTheme | //@w:themeColor") > 0
        and count(s, "word/styles.xml", "//@w:asciiTheme | //@w:themeColor") == 0,
    },
    "docx-tema-parcasi": {
        "ext": "docx",
        "what": "the theme part is rewritten minimal: per-script fonts and gradient/shadow/3-D effect styles are dropped",
        "where": "WordCraft DOCX writer (upstream)",
        "probe": lambda o, s: count(o, "word/theme/theme1.xml", "//a:fontScheme//a:font | //a:fmtScheme//a:gradFill") > 0
        and count(s, "word/theme/theme1.xml", "//a:fontScheme//a:font | //a:fmtScheme//a:gradFill") == 0,
    },
    "docx-tablo-stili-paragraf": {
        "ext": "docx",
        "what": "paragraph spacing inside table styles' conditional parts (first row, last row…) is dropped",
        "where": "WordCraft DOCX writer (upstream)",
        "probe": lambda o, s: count(o, "word/styles.xml", "//w:tblStylePr/w:pPr") > 0 and count(s, "word/styles.xml", "//w:tblStylePr/w:pPr") == 0,
    },
    "docx-resim-oran-kilidi": {
        "ext": "docx",
        "what": "a picture's aspect-ratio lock (graphicFrameLocks noChangeAspect) is dropped",
        "where": "WordCraft DOCX writer (upstream)",
        "probe": lambda o, s: count(o, "word/document.xml", "//a:graphicFrameLocks[@noChangeAspect='1']") > 0
        and count(s, "word/document.xml", "//a:graphicFrameLocks") == 0,
    },
    "pptx-yazici-ayarlari": {
        "ext": "pptx",
        "what": "the printer settings part (ppt/printerSettings/*.bin) is dropped",
        "where": "DeckCraft PPTX writer (upstream)",
        "part": "application/vnd.openxmlformats-officedocument.presentationml.printerSettings",
        "probe": lambda o, s: any(n.startswith("ppt/printerSettings/") for n in parts(o)) and not any(n.startswith("ppt/printerSettings/") for n in parts(s)),
    },
    "pptx-kucuk-resim": {
        "ext": "pptx",
        "what": "the file's preview picture (docProps/thumbnail.jpeg) is dropped",
        "where": "DeckCraft PPTX writer (upstream)",
        "part": "image/jpeg",
        "probe": lambda o, s: "docProps/thumbnail.jpeg" in parts(o) and "docProps/thumbnail.jpeg" not in parts(s),
    },
}


def generate(root):
    root.mkdir(parents=True, exist_ok=True)
    picture = root / "resim.png"
    picture.write_bytes(png())
    make_docx(root / "korpus.docx", picture)
    make_xlsx(root / "korpus.xlsx")
    make_pptx(root / "korpus.pptx", picture)
    picture.unlink()


def check(root):
    problems, known_seen, checked = [], [], 0
    for ext, fn in (("docx", check_docx), ("xlsx", check_xlsx), ("pptx", check_pptx)):
        for orig in sorted(root.glob(f"*.{ext}")):
            if orig.name.endswith(f".saved.{ext}"):
                continue
            saved = orig.with_name(f"{orig.stem}.saved.{ext}")
            if not saved.exists():
                problems.append(f"{saved.name}: not written by the engine round trip")
                continue
            # What the bridge itself reported it could not keep (none is expected here).
            warnings = orig.with_name(f"{orig.name}.warnings.txt")
            reported = warnings.read_text(encoding="utf-8").strip() if warnings.exists() else ""
            expect(not reported, f"{orig.name}: the engine reported losses: {reported}", problems)
            fn(orig, saved, problems)
            known = []
            for key, loss in KNOWN_LOSSES.items():
                if loss["ext"] != ext:
                    continue
                if loss["probe"](orig, saved):
                    known.append(key)
                else:
                    problems.append(f"{orig.name}: known loss {key!r} no longer happens ({loss['what']}); update KNOWN_LOSSES and docs/KAYIPSIZ-KAYIT-KORPUSU.md")
            check_package(orig, saved, problems, known)
            known_seen += [(orig.name, k) for k in known]
            checked += 1
    if checked != 3:
        problems.append(f"expected 3 corpus files, checked {checked}")
    for name, key in known_seen:
        print(f"  known loss  {name}: {key}: {KNOWN_LOSSES[key]['what']} [{KNOWN_LOSSES[key]['where']}]")
    for p in problems:
        print(f"  FAIL  {p}")
    if problems:
        print(f"Lossless-save corpus: {len(problems)} problem(s); see docs/KAYIPSIZ-KAYIT-KORPUSU.md", file=sys.stderr)
        sys.exit(1)
    print(f"Lossless-save corpus: {checked} files opened, edited, saved and re-read; nothing lost beyond the {len(known_seen)} known losses.")


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] not in ("generate", "check"):
        sys.exit("usage: office_corpus.py generate|check DIR")
    (generate if sys.argv[1] == "generate" else check)(pathlib.Path(sys.argv[2]))
