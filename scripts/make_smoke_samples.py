#!/usr/bin/env python3
"""Sample documents for the emulator smoke test of the release APK (scripts/emulator-smoke.sh).

`make_smoke_samples.py [--small-only] DIR` writes one small file per format the app opens into DIR,
a few large ones for the performance baseline (skipped with --small-only), plus `samples.tsv` that
tells the smoke test how to hand each one to the app:

    file name <TAB> MIME type <TAB> route <TAB> expectation <TAB> size class

route: `filter` sends an implicit VIEW intent restricted to the app's package, so the manifest's
intent filter has to match; `explicit` names MainActivity directly, for types the filter does not
list (the app still opens them from its own picker and file list).
expectation: `viewer` means a document screen must appear; `viewer-or-notice` also accepts the
app's "this format opens in an upcoming version" notice.
size class: `small`, or `big` for the large generated files, which get a longer open timeout and
a larger soft time budget in the smoke test.

`samples-tr.tsv` lists what the smoke test's second pass opens (Turkish locale, font scale 2.0,
dark theme): a subset of the files above plus copies whose names are written the way people name
files here: upper-case extensions, Turkish letters, a space. An extension containing "I" (.ZIP,
.ICS) is lower-cased to "ı" by a locale-sensitive conversion under tr-TR, which is the bug class
this pass is for. Its rows have a sixth column, the ASCII step name used in the smoke test's
output files.

Legacy binary formats: only .xls has a file in the repository's test data (mo-grid); there is no
.doc or .ppt file to copy, so those two are not covered here.

Everything is written here with the Python standard library or copied from test data that is
already in the repository: nothing is downloaded and no binary file is committed.
"""

import pathlib
import random
import re
import shutil
import struct
import sys
import unicodedata
import zipfile
import zlib

REPO = pathlib.Path(__file__).resolve().parent.parent
TURKISH = "İstanbul'da ılık bir gün: Şişli, Ağrı, Üsküdar, Ödemiş, Çeşme — ığüşöç İĞÜŞÖÇ."

# Every name in the first pass starts with "mo-smoke-" (its step names are derived from it).
SAMPLES = []  # (name, mime, route, expectation, size)

# Second pass (tr-TR, font scale 2.0, dark): (step, existing sample) for the formats it repeats,
# then (new name, copied from, step) for the names an ASCII, en-US test never sees. NFC, as a
# Turkish keyboard writes them.
TR_SUBSET = [
    ("tr-docx", "mo-smoke-docx.docx"),
    ("tr-xlsx", "mo-smoke-xlsx.xlsx"),
    ("tr-pptx", "mo-smoke-pptx.pptx"),
    ("tr-pdf", "mo-smoke-pdf.pdf"),
    ("tr-txt", "mo-smoke-txt.txt"),
    ("tr-csv", "mo-smoke-csv.csv"),
    ("tr-png", "mo-smoke-png.png"),
    ("tr-epub", "mo-smoke-epub.epub"),
]
TR_RENAMED = [
    ("RAPOR-İŞ.DOCX", "mo-smoke-docx.docx", "tr-upper-docx"),
    ("TABLO_ı.XLSX", "mo-smoke-xlsx.xlsx", "tr-upper-xlsx"),
    ("SUNUM.PPTX", "mo-smoke-pptx.pptx", "tr-upper-pptx"),
    ("BELGE.PDF", "mo-smoke-pdf.pdf", "tr-upper-pdf"),
    ("NOT_ğüşöç.TXT", "mo-smoke-txt.txt", "tr-upper-txt"),
    ("GİDER LİSTESİ.CSV", "mo-smoke-csv.csv", "tr-upper-csv"),
    # "ZIP".lowercase(tr) is "zıp", "ICS" is "ıcs": routing by extension has to use Locale.ROOT.
    ("ARŞİV.ZIP", "mo-smoke-zip.zip", "tr-upper-zip"),
    ("TAKVİM.ICS", "mo-smoke-ics.ics", "tr-upper-ics"),
]


def sample(name, mime, route="filter", expect="viewer", size="small"):
    SAMPLES.append((name, mime, route, expect, size))
    return name


def write_zip(path, entries, first_stored=None):
    """A ZIP with `first_stored` (name, bytes) stored uncompressed first, as ODF and EPUB require."""
    with zipfile.ZipFile(path, "w") as z:
        if first_stored:
            z.writestr(zipfile.ZipInfo(first_stored[0]), first_stored[1], compress_type=zipfile.ZIP_STORED)
        for name, data in entries:
            z.writestr(name, data, compress_type=zipfile.ZIP_DEFLATED)


def xml_escape(text):
    return text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


# --------------------------------------------------------------------------------------------
# Word processing


def make_docx(path):
    paragraphs = [
        ("Heading1", "Duman testi"),
        (None, TURKISH),
        (None, "Bu belge sürüm derlemesinin açılıp açılmadığını denemek için üretildi."),
    ]
    write_docx(path, "".join(docx_paragraph(text, style) for style, text in paragraphs))


def docx_paragraph(text, style=None, page_break_before=False):
    return (
        "<w:p>"
        + (f'<w:pPr><w:pStyle w:val="{style}"/></w:pPr>' if style else "")
        + ('<w:r><w:br w:type="page"/></w:r>' if page_break_before else "")
        + f'<w:r><w:t xml:space="preserve">{xml_escape(text)}</w:t></w:r></w:p>'
    )


TABLE_GRID_STYLE = (
    '<w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:tblPr><w:tblBorders>'
    + "".join(
        f'<w:{side} w:val="single" w:sz="4" w:space="0" w:color="000000"/>'
        for side in ("top", "left", "bottom", "right", "insideH", "insideV")
    )
    + "</w:tblBorders></w:tblPr></w:style>"
)


def write_docx(path, body, table_style=False):
    """A minimal DOCX package (document and styles) whose body holds `body`; `table_style` adds a
    bordered "TableGrid" table style."""
    w = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
    write_zip(
        path,
        [
            (
                "[Content_Types].xml",
                '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                '<Default Extension="xml" ContentType="application/xml"/>'
                '<Override PartName="/word/document.xml" '
                'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
                '<Override PartName="/word/styles.xml" '
                'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>'
                "</Types>",
            ),
            (
                "_rels/.rels",
                '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                '<Relationship Id="rId1" '
                'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" '
                'Target="word/document.xml"/></Relationships>',
            ),
            (
                "word/_rels/document.xml.rels",
                '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                '<Relationship Id="rId1" '
                'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" '
                'Target="styles.xml"/></Relationships>',
            ),
            (
                "word/styles.xml",
                f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles {w}>'
                '<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>'
                '<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/>'
                '<w:basedOn w:val="Normal"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr>'
                '<w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style>'
                + (TABLE_GRID_STYLE if table_style else "")
                + "</w:styles>",
            ),
            (
                "word/document.xml",
                f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {w}><w:body>{body}'
                '<w:sectPr><w:pgSz w:w="11906" w:h="16838"/>'
                '<w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr>'
                "</w:body></w:document>",
            ),
        ],
    )


def make_odf(path, mime, body):
    """A minimal ODF package (ODT/ODS) whose content.xml holds `body`."""
    ns = (
        'xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" '
        'xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" '
        'xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" '
        'office:version="1.3"'
    )
    write_zip(
        path,
        [
            (
                "META-INF/manifest.xml",
                '<?xml version="1.0" encoding="UTF-8"?>'
                '<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" '
                'manifest:version="1.3">'
                f'<manifest:file-entry manifest:full-path="/" manifest:media-type="{mime}"/>'
                '<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>'
                "</manifest:manifest>",
            ),
            (
                "content.xml",
                f'<?xml version="1.0" encoding="UTF-8"?><office:document-content {ns}>'
                f"<office:body>{body}</office:body></office:document-content>",
            ),
        ],
        first_stored=("mimetype", mime),
    )


def make_odt(path):
    make_odf(
        path,
        "application/vnd.oasis.opendocument.text",
        "<office:text>"
        '<text:h text:outline-level="1">Duman testi</text:h>'
        f"<text:p>{xml_escape(TURKISH)}</text:p>"
        "</office:text>",
    )


def make_rtf(path):
    # RTF is 8-bit: Turkish letters as \uN escapes with a "?" fallback.
    escaped = "".join(c if ord(c) < 128 else f"\\u{ord(c) if ord(c) < 32768 else ord(c) - 65536}?" for c in TURKISH)
    path.write_text(
        "{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Arial;}}\\f0\\fs24 {\\b Duman testi}\\par " + escaped + "\\par }\n",
        encoding="ascii",
    )


def make_epub(path):
    xhtml = (
        '<?xml version="1.0" encoding="UTF-8"?><html xmlns="http://www.w3.org/1999/xhtml" '
        'xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="tr"><head><title>{title}</title></head>'
        "<body>{body}</body></html>"
    )
    write_zip(
        path,
        [
            (
                "META-INF/container.xml",
                '<?xml version="1.0" encoding="UTF-8"?>'
                '<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">'
                '<rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>'
                "</rootfiles></container>",
            ),
            (
                "OEBPS/content.opf",
                '<?xml version="1.0" encoding="UTF-8"?>'
                '<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">'
                '<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">'
                '<dc:identifier id="id">urn:uuid:6f1c1b2e-2f43-4d7c-9c55-0b1d6c1e0a01</dc:identifier>'
                "<dc:title>Duman testi</dc:title><dc:language>tr</dc:language>"
                '<meta property="dcterms:modified">2026-01-01T00:00:00Z</meta></metadata>'
                '<manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>'
                '<item id="c1" href="chapter1.xhtml" media-type="application/xhtml+xml"/></manifest>'
                '<spine><itemref idref="c1"/></spine></package>',
            ),
            (
                "OEBPS/nav.xhtml",
                xhtml.format(
                    title="İçindekiler",
                    body='<nav epub:type="toc"><ol><li><a href="chapter1.xhtml">Bölüm 1</a></li></ol></nav>',
                ),
            ),
            (
                "OEBPS/chapter1.xhtml",
                xhtml.format(title="Bölüm 1", body=f"<h1>Bölüm 1</h1><p>{xml_escape(TURKISH)}</p>"),
            ),
        ],
        first_stored=("mimetype", "application/epub+zip"),
    )


def make_eml(path):
    path.write_bytes(
        (
            "From: Ayşe Yılmaz <ayse@example.invalid>\r\n"
            "To: Mehmet Öztürk <mehmet@example.invalid>\r\n"
            "Subject: =?UTF-8?B?RHVtYW4gdGVzdGk=?=\r\n"
            "Date: Thu, 01 Jan 2026 10:00:00 +0300\r\n"
            "MIME-Version: 1.0\r\n"
            "Content-Type: text/plain; charset=UTF-8\r\n"
            "Content-Transfer-Encoding: 8bit\r\n"
            "\r\n"
            f"Merhaba,\r\n\r\n{TURKISH}\r\n"
        ).encode("utf-8")
    )


def make_vcf(path):
    path.write_bytes(
        (
            "BEGIN:VCARD\r\nVERSION:3.0\r\nN:Yılmaz;Ayşe;;;\r\nFN:Ayşe Yılmaz\r\n"
            "TEL;TYPE=CELL:+90 555 000 00 00\r\nEMAIL:ayse@example.invalid\r\n"
            "ADR;TYPE=WORK:;;Şişli Cd. 1;İstanbul;;34000;Türkiye\r\nEND:VCARD\r\n"
        ).encode("utf-8")
    )


def make_ics(path):
    path.write_bytes(
        (
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//mobileoffice-free//smoke//TR\r\n"
            "BEGIN:VEVENT\r\nUID:smoke-1@example.invalid\r\nDTSTAMP:20260101T080000Z\r\n"
            "DTSTART:20260105T090000Z\r\nDTEND:20260105T100000Z\r\nSUMMARY:Toplantı: Üsküdar şubesi\r\n"
            "LOCATION:İstanbul\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
        ).encode("utf-8")
    )


# --------------------------------------------------------------------------------------------
# Spreadsheets


def make_xlsx(path):
    strings = ["Şehir", "Nüfus", "İstanbul", "Ağrı", "Toplam"]
    rows = [
        [("s", 0), ("s", 1)],
        [("s", 2), ("n", 15655924)],
        [("s", 3), ("n", 510626)],
        [("s", 4), ("f", "SUM(B2:B3)")],
    ]

    def cell(ref, kind, value):
        if kind == "s":
            return f'<c r="{ref}" t="s"><v>{value}</v></c>'
        if kind == "f":
            return f'<c r="{ref}"><f>{value}</f></c>'
        return f'<c r="{ref}"><v>{value}</v></c>'

    sheet_rows = "".join(
        f'<row r="{r + 1}">' + "".join(cell(f"{'AB'[c]}{r + 1}", k, v) for c, (k, v) in enumerate(row)) + "</row>"
        for r, row in enumerate(rows)
    )
    write_xlsx(path, sheet_rows, strings)


def write_xlsx(path, sheet_rows, strings, sheet_name="Veri"):
    """A minimal XLSX package with one sheet whose <sheetData> holds `sheet_rows`."""
    main = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
    rel = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
    ct = "application/vnd.openxmlformats-officedocument.spreadsheetml"
    rt = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
    write_zip(
        path,
        [
            (
                "[Content_Types].xml",
                '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                '<Default Extension="xml" ContentType="application/xml"/>'
                f'<Override PartName="/xl/workbook.xml" ContentType="{ct}.sheet.main+xml"/>'
                f'<Override PartName="/xl/worksheets/sheet1.xml" ContentType="{ct}.worksheet+xml"/>'
                f'<Override PartName="/xl/sharedStrings.xml" ContentType="{ct}.sharedStrings+xml"/>'
                f'<Override PartName="/xl/styles.xml" ContentType="{ct}.styles+xml"/>'
                "</Types>",
            ),
            (
                "_rels/.rels",
                '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                f'<Relationship Id="rId1" Type="{rt}/officeDocument" Target="xl/workbook.xml"/></Relationships>',
            ),
            (
                "xl/workbook.xml",
                f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook {main} {rel}>'
                f'<sheets><sheet name="{xml_escape(sheet_name)}" sheetId="1" r:id="rId1"/></sheets></workbook>',
            ),
            (
                "xl/_rels/workbook.xml.rels",
                '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                f'<Relationship Id="rId1" Type="{rt}/worksheet" Target="worksheets/sheet1.xml"/>'
                f'<Relationship Id="rId2" Type="{rt}/sharedStrings" Target="sharedStrings.xml"/>'
                f'<Relationship Id="rId3" Type="{rt}/styles" Target="styles.xml"/></Relationships>',
            ),
            (
                "xl/worksheets/sheet1.xml",
                f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet {main}>'
                f"<sheetData>{sheet_rows}</sheetData></worksheet>",
            ),
            (
                "xl/sharedStrings.xml",
                f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                f'<sst {main} count="{len(strings)}" uniqueCount="{len(strings)}">'
                + "".join(f"<si><t>{xml_escape(s)}</t></si>" for s in strings)
                + "</sst>",
            ),
            (
                "xl/styles.xml",
                f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet {main}>'
                '<fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>'
                '<fills count="2"><fill><patternFill patternType="none"/></fill>'
                '<fill><patternFill patternType="gray125"/></fill></fills>'
                '<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>'
                '<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>'
                '<cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs>'
                '<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>'
                "</styleSheet>",
            ),
        ],
    )


def make_csv(path):
    path.write_text("Şehir,Nüfus\nİstanbul,15655924\nAğrı,510626\n", encoding="utf-8")


# --------------------------------------------------------------------------------------------
# PDF, picture, text, archive


def make_pdf(path):
    """A one-page PDF with real text, written by hand with a correct cross-reference table."""
    content = b"BT /F1 24 Tf 72 760 Td (Duman testi - smoke test) Tj 0 -36 Td /F1 14 Tf (Istanbul, Agri, Uskudar) Tj ET"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        b"<< /Length " + str(len(content)).encode() + b" >>\nstream\n" + content + b"\nendstream",
    ]
    write_pdf(path, objects)


def write_pdf(path, objects):
    """A PDF of `objects` (object n is objects[n - 1], 1 the catalog) with a correct xref table."""
    out = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += f"{number} 0 obj\n".encode() + body + b"\nendobj\n"
    xref = len(out)
    out += f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode()
    out += b"".join(f"{o:010d} 00000 n \n".encode() for o in offsets)
    out += f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    path.write_bytes(bytes(out))


def png_bytes(width, height, pixel):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    rows = b"".join(b"\x00" + b"".join(bytes(pixel(x, y)) for x in range(width)) for y in range(height))
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def make_png(path):
    # Dark bars on white, like lines of print: enough for the OCR engine to have something to read.
    def pixel(x, y):
        line = (y - 20) % 40 < 14 and 20 <= y < 220 and 24 <= x < 24 + 280 - (y // 40) * 40
        return (25, 25, 25) if line else (250, 250, 250)

    path.write_bytes(png_bytes(320, 240, pixel))


def make_txt(path):
    path.write_text(f"Duman testi\n\n{TURKISH}\nSatır 4\n", encoding="utf-8")


def make_md(path):
    path.write_text(f"# Duman testi\n\n{TURKISH}\n\n- madde bir\n- madde iki\n", encoding="utf-8")


def make_zip(path, docx, txt):
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        z.write(docx, "belgeler/rapor.docx")
        z.write(txt, "notlar.txt")


# --------------------------------------------------------------------------------------------
# Large files for the performance baseline. Fixed seeds: the same bytes on every run, so open
# times can be compared between runs.

WORDS = (
    "belge rapor tablo satır sütun hücre toplam bütçe şirket müşteri sipariş fatura ödeme tarih "
    "İstanbul Ankara İzmir Bursa Ağrı Çorum Muğla Şanlıurfa Iğdır Ödemiş Üsküdar ılık güneşli "
    "toplantı karar öneri değerlendirme çalışma süreç hizmet ürün kalite güvenlik eğitim proje "
    "yıllık aylık haftalık günlük önemli gerekli yeni eski büyük küçük hızlı yavaş açık kapalı "
    "ve ile için gibi kadar sonra önce üzerinde altında arasında göre rağmen dolayı birlikte"
).split()


def sentence(rng, low=8, high=16):
    words = [rng.choice(WORDS) for _ in range(rng.randint(low, high))]
    text = " ".join(words)
    return text[:1].upper() + text[1:] + "."


def make_big_xlsx(path, rows=45_000):
    """`rows` data rows x 10 columns: numbers, shared strings, every 500th amount a formula with its
    cached value, and a total row with two SUMs over the whole sheet.

    45 000 rows, not 50 000: the spreadsheet engine reads at most 1 000 000 XML elements from a
    sheet part and reports the rest as damaged; 50 000 rows x 10 cells (<c> and <v> each) is just
    over that and would be cut at 49 995 rows. This file stays below it to measure a clean open."""
    rng = random.Random(45_000)
    header = ["No", "Şehir", "Kategori", "Adet", "Birim fiyat", "Tutar", "İndirim %", "Kod", "Tarih", "Not"]
    cities = ["İstanbul", "Ankara", "İzmir", "Bursa", "Ağrı", "Çorum", "Muğla", "Şanlıurfa", "Iğdır", "Ödemiş"]
    categories = [f"Kategori {i}: {w}" for i, w in enumerate(WORDS[:30])]
    codes = [f"KOD-{i:04d}" for i in range(2000)]
    notes = sorted({sentence(rng, 3, 7) for _ in range(300)})
    strings = header + cities + categories + codes + notes + ["Toplam"]
    index = {text: i for i, text in enumerate(strings)}

    def s(ref, text):
        return f'<c r="{ref}" t="s"><v>{index[text]}</v></c>'

    def n(ref, value):
        return f'<c r="{ref}"><v>{value}</v></c>'

    out = ['<row r="1">' + "".join(s(f"{chr(65 + c)}1", h) for c, h in enumerate(header)) + "</row>"]
    total_qty = 0
    total_amount = 0.0
    for r in range(2, rows + 2):
        qty = rng.randint(1, 500)
        price = round(rng.uniform(1, 2000), 2)
        amount = qty * price
        total_qty += qty
        total_amount += amount
        if r % 500 == 0:
            amount_cell = f'<c r="F{r}"><f>D{r}*E{r}</f><v>{amount!r}</v></c>'
        else:
            amount_cell = n(f"F{r}", repr(amount))
        out.append(
            f'<row r="{r}">'
            + n(f"A{r}", r - 1)
            + s(f"B{r}", rng.choice(cities))
            + s(f"C{r}", rng.choice(categories))
            + n(f"D{r}", qty)
            + n(f"E{r}", price)
            + amount_cell
            + n(f"G{r}", rng.choice((0, 5, 10, 12.5, 20)))
            + s(f"H{r}", rng.choice(codes))
            + n(f"I{r}", 45_000 + rng.randint(0, 1500))
            + s(f"J{r}", rng.choice(notes))
            + "</row>"
        )
    last, t = rows + 1, rows + 2
    out.append(
        f'<row r="{t}">'
        + s(f"A{t}", "Toplam")
        + f'<c r="D{t}"><f>SUM(D2:D{last})</f><v>{total_qty}</v></c>'
        + f'<c r="F{t}"><f>SUM(F2:F{last})</f><v>{total_amount!r}</v></c>'
        + "</row>"
    )
    write_xlsx(path, "".join(out), strings, sheet_name="Büyük veri")


def make_big_docx(path, pages=300):
    """`pages` chapters, each a heading on a new page and about 300 words, so the document is at
    least `pages` pages long; six chapters carry a 4-column table instead of part of their text."""
    rng = random.Random(300)
    cell = (
        '<w:tc><w:tcPr><w:tcW w:w="2256" w:type="dxa"/></w:tcPr>'
        '<w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:tc>'
    )
    parts = []
    for chapter in range(1, pages + 1):
        parts.append(docx_paragraph(f"Bölüm {chapter}", "Heading1", page_break_before=chapter > 1))
        with_table = chapter % 50 == 25
        for _ in range(2 if with_table else 4):
            parts.append(docx_paragraph(" ".join(sentence(rng) for _ in range(6))))
        if with_table:
            rows = [["Şehir", "Adet", "Tutar", "Not"]] + [
                [rng.choice(WORDS).capitalize(), str(rng.randint(1, 99)), f"{rng.uniform(1, 9999):.2f}", rng.choice(WORDS)]
                for _ in range(12)
            ]
            parts.append(
                '<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="0" w:type="auto"/></w:tblPr>'
                + "<w:tblGrid>"
                + '<w:gridCol w:w="2256"/>' * 4
                + "</w:tblGrid>"
                + "".join("<w:tr>" + "".join(cell.format(xml_escape(v)) for v in row) + "</w:tr>" for row in rows)
                + "</w:tbl>"
            )
            parts.append(docx_paragraph(sentence(rng)))
    write_docx(path, "".join(parts), table_style=True)


def make_big_pdf(path, pages=200):
    """`pages` A4 pages of Helvetica text, one content stream per page, sharing one font."""
    rng = random.Random(200)
    ascii_words = [w for w in WORDS if w.isascii()]
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"",  # the page tree, written once the pages are numbered
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    ]
    kids = []
    for page in range(1, pages + 1):
        lines = [f"BT /F1 18 Tf 56 790 Td (Sayfa {page} / {pages}) Tj /F1 11 Tf".encode()]
        for _ in range(48):
            text = " ".join(rng.choice(ascii_words) for _ in range(rng.randint(8, 13)))
            lines.append(f"0 -15 Td ({text}) Tj".encode())
        lines.append(b"ET")
        content = b"\n".join(lines)
        number = len(objects) + 1
        kids.append(f"{number} 0 R")
        objects.append(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> "
            + f"/Contents {number + 1} 0 R >>".encode()
        )
        objects.append(b"<< /Length " + str(len(content)).encode() + b" >>\nstream\n" + content + b"\nendstream")
    objects[1] = f"<< /Type /Pages /Kids [{' '.join(kids)}] /Count {pages} >>".encode()
    write_pdf(path, objects)


def make_big_pptx(path, source, slides=60):
    """`source`'s one slide copied `slides` times (titles numbered), every copy listed in
    [Content_Types].xml, the presentation's relationships and its slide list."""
    with zipfile.ZipFile(source) as z:
        parts = {info.filename: z.read(info.filename) for info in z.infolist()}
    slide = parts["ppt/slides/slide1.xml"].decode("utf-8")
    slide_rels = parts["ppt/slides/_rels/slide1.xml.rels"]
    types = parts["[Content_Types].xml"].decode("utf-8")
    rels = parts["ppt/_rels/presentation.xml.rels"].decode("utf-8")
    presentation = parts["ppt/presentation.xml"].decode("utf-8")
    first = re.search(r'Id="(rId\d+)"[^>]*Target="slides/slide1\.xml"', rels)
    if slide.count("<a:t>Başlık</a:t>") != 1 or presentation.count("<p:sldId ") != 1 or not first:
        sys.exit(f"{source} no longer has the one-slide layout make_big_pptx expects")

    slide_type = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml"
    rel_type = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide"
    next_rel = max(int(i) for i in re.findall(r'Id="rId(\d+)"', rels)) + 1
    slide_ids = [f'<p:sldId id="256" r:id="{first.group(1)}"/>']
    for number in range(1, slides + 1):
        parts[f"ppt/slides/slide{number}.xml"] = slide.replace(
            "<a:t>Başlık</a:t>", f"<a:t>Başlık {number} / {slides}</a:t>"
        ).encode("utf-8")
        if number == 1:
            continue
        parts[f"ppt/slides/_rels/slide{number}.xml.rels"] = slide_rels
        types = types.replace(
            "</Types>", f'<Override PartName="/ppt/slides/slide{number}.xml" ContentType="{slide_type}"/></Types>'
        )
        rels = rels.replace(
            "</Relationships>",
            f'<Relationship Id="rId{next_rel}" Type="{rel_type}" Target="slides/slide{number}.xml"/></Relationships>',
        )
        slide_ids.append(f'<p:sldId id="{255 + number}" r:id="rId{next_rel}"/>')
        next_rel += 1
    presentation = re.sub(
        r"<p:sldIdLst>.*?</p:sldIdLst>", f"<p:sldIdLst>{''.join(slide_ids)}</p:sldIdLst>", presentation
    )
    parts["[Content_Types].xml"] = types.encode("utf-8")
    parts["ppt/_rels/presentation.xml.rels"] = rels.encode("utf-8")
    parts["ppt/presentation.xml"] = presentation.encode("utf-8")
    # [Content_Types].xml first, as Office writes it; the other parts in the source's order.
    write_zip(path, sorted(parts.items(), key=lambda item: item[0] != "[Content_Types].xml"))


# --------------------------------------------------------------------------------------------


def main(out, big=True):
    out.mkdir(parents=True, exist_ok=True)

    def p(name):
        return out / name

    make_docx(p(sample("mo-smoke-docx.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document")))
    make_odt(p(sample("mo-smoke-odt.odt", "application/vnd.oasis.opendocument.text")))
    make_rtf(p(sample("mo-smoke-rtf.rtf", "application/rtf")))
    make_md(p(sample("mo-smoke-md.md", "text/markdown")))
    # Read as documents through mo-convert; the VIEW filter does not list these types.
    make_epub(p(sample("mo-smoke-epub.epub", "application/epub+zip", route="explicit")))
    make_eml(p(sample("mo-smoke-eml.eml", "message/rfc822", route="explicit")))
    make_vcf(p(sample("mo-smoke-vcf.vcf", "text/vcard", route="explicit")))
    make_ics(p(sample("mo-smoke-ics.ics", "text/calendar", route="explicit")))
    make_xlsx(p(sample("mo-smoke-xlsx.xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")))
    make_csv(p(sample("mo-smoke-csv.csv", "text/csv")))
    # The legacy binary formats the app routes (.doc, .xls, .ppt): only .xls has a test file.
    shutil.copyfile(REPO / "rust/mo-grid/testdata/styled.xls", p(sample("mo-smoke-xls.xls", "application/vnd.ms-excel")))
    shutil.copyfile(
        REPO / "rust/mo-deck/testdata/grouped-text.pptx",
        p(sample("mo-smoke-pptx.pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation")),
    )
    shutil.copyfile(
        REPO / "rust/mo-deck/testdata/sample.odp",
        p(sample("mo-smoke-odp.odp", "application/vnd.oasis.opendocument.presentation")),
    )
    make_pdf(p(sample("mo-smoke-pdf.pdf", "application/pdf")))
    shutil.copyfile(REPO / "rust/mo-pdf/testdata/jspdf-form.pdf", p(sample("mo-smoke-pdfform.pdf", "application/pdf")))
    shutil.copyfile(REPO / "rust/mo-cad/tests/data/apartment.dxf", p(sample("mo-smoke-dxf.dxf", "image/vnd.dxf")))
    make_png(p(sample("mo-smoke-png.png", "image/png")))
    make_txt(p(sample("mo-smoke-txt.txt", "text/plain")))
    make_zip(p(sample("mo-smoke-zip.zip", "application/zip")), out / "mo-smoke-docx.docx", out / "mo-smoke-txt.txt")

    if big:
        # Last, so a slow or failing large file cannot hide a regression in the small ones.
        make_big_xlsx(
            p(sample("mo-smoke-big-xlsx.xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", size="big"))
        )
        make_big_docx(
            p(sample("mo-smoke-big-docx.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", size="big"))
        )
        make_big_pdf(p(sample("mo-smoke-big-pdf.pdf", "application/pdf", size="big")))
        make_big_pptx(
            p(sample("mo-smoke-big-pptx.pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation", size="big")),
            REPO / "rust/mo-deck/testdata/grouped-text.pptx",
        )

    (out / "samples.tsv").write_text("".join("\t".join(s) + "\n" for s in SAMPLES), encoding="utf-8")
    for name, mime, route, expect, size in SAMPLES:
        print(f"{name:24} {(out / name).stat().st_size:>9} B  {route:8} {expect:16} {size:5} {mime}")

    by_name = {s[0]: s for s in SAMPLES}
    tr_rows = [(*by_name[name], step) for step, name in TR_SUBSET]
    for name, source, step in TR_RENAMED:
        name = unicodedata.normalize("NFC", name)
        shutil.copyfile(out / source, out / name)
        tr_rows.append((name, *by_name[source][1:], step))
    (out / "samples-tr.tsv").write_text("".join("\t".join(r) + "\n" for r in tr_rows), encoding="utf-8")
    print("second pass (tr-TR, font scale 2.0, dark):")
    for name, mime, route, expect, size, step in tr_rows:
        print(f"{name:24} {(out / name).stat().st_size:>9} B  {route:8} {expect:16} {step:14} {mime}")


if __name__ == "__main__":
    args = sys.argv[1:]
    small_only = "--small-only" in args
    args = [a for a in args if a != "--small-only"]
    if len(args) != 1 or args[0].startswith("-"):
        sys.exit(f"usage: {sys.argv[0]} [--small-only] OUTPUT_DIR")
    main(pathlib.Path(args[0]), big=not small_only)
