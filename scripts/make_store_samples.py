#!/usr/bin/env python3
"""Presentable sample documents for the Google Play phone screenshots (docs/GOOGLE-PLAY.md).

`make_store_samples.py DIR` writes, for each store language (tr-TR, en-US), a short report (DOCX)
with a heading and a table, a household budget (XLSX), a three-slide presentation (PPTX), one page
of meeting minutes (PDF) and a few small files that only fill the home screen's list (TXT, CSV,
MD), into DIR/<locale>/, plus DIR/shots.tsv, which tells scripts/emulator-smoke.sh's last pass
("Mağaza ekran görüntüleri") what to open and what to call each screenshot:

    locale <TAB> screenshot name (fastlane order, "-" for list-only files) <TAB> file name <TAB> MIME type

The content is made up: no real person, company or address, and nothing that names the
upstream engines. Everything is written here with the Python standard library; the PDF uses the
standard Helvetica fonts every PDF reader has (Turkish letters through a /Differences encoding),
the Office files name Calibri, which the app replaces with its bundled, OFL-licensed Carlito. The
slide master and theme come from rust/mo-deck/testdata/grouped-text.pptx, which is already in
the repository; its author metadata and thumbnail are dropped. Nothing is downloaded and no
binary file is committed.
"""

import pathlib
import re
import sys
import unicodedata
import zipfile

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from make_smoke_samples import REPO, write_pdf, write_zip, xml_escape  # noqa: E402

DOCX_MIME = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
XLSX_MIME = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
PPTX_MIME = "application/vnd.openxmlformats-officedocument.presentationml.presentation"

NAVY = "1F4E79"
BLUE = "2E75B6"
LIGHT = "DDEBF7"

# Everything that differs between the languages. Amounts are made up; "money" formats them.
CONTENT = {
    "tr-TR": {
        "files": {
            "docx": "Faaliyet Raporu.docx",
            "xlsx": "Aile Bütçesi 2026.xlsx",
            "pptx": "Satış Sunumu.pptx",
            "pdf": "Toplantı Tutanağı.pdf",
            "txt": "Alışveriş Listesi.txt",
            "csv": "Stok Sayımı.csv",
            "md": "Proje Planı.md",
        },
        "report": {
            "title": "Üçüncü Çeyrek Faaliyet Raporu",
            "subtitle": "Kuzey Bölge Ofisi · Ekim 2026",
            "sections": [
                ("Özet", [
                    "Üçüncü çeyrekte satışlar hedefin yüzde 6 üzerinde gerçekleşti. Yeni müşteri sayısı "
                    "geçen yılın aynı dönemine göre yüzde 14 arttı; iade oranı yüzde 2,1'e geriledi.",
                ]),
                ("Satış sonuçları", "TABLE"),
                ("Sonraki adımlar", [
                    "• Dördüncü çeyrek kampanya takvimini kasım başında kesinleştirmek.",
                    "• Depo stok sayımını yıl sonundan önce tamamlamak.",
                    "• Müşteri memnuniyeti anketinin sonuçlarını ekiple paylaşmak.",
                ]),
            ],
            "table_head": ["Ürün grubu", "Hedef", "Gerçekleşen", "Oran"],
            "table_rows": [
                ["Ofis mobilyası", "1.200.000 TL", "1.315.000 TL", "%110"],
                ["Kırtasiye", "450.000 TL", "438.500 TL", "%97"],
                ["Bilgisayar ve donanım", "2.100.000 TL", "2.240.000 TL", "%107"],
                ["Teknik servis", "380.000 TL", "402.000 TL", "%106"],
            ],
            "table_total": ["Toplam", "4.130.000 TL", "4.395.500 TL", "%106"],
        },
        "budget": {
            "sheet": "Bütçe",
            "title": "Aylık Ev Bütçesi · Kasım 2026",
            "head": ["Kalem", "Planlanan", "Gerçekleşen", "Fark"],
            "rows": [
                ("Kira", 18000, 18000),
                ("Market", 9500, 10240),
                ("Elektrik, su, doğalgaz", 3200, 2870),
                ("Ulaşım", 2500, 2310),
                ("İnternet ve telefon", 900, 900),
                ("Eğitim", 4000, 4000),
                ("Sağlık", 1000, 650),
                ("Eğlence", 2000, 2460),
                ("Birikim", 6000, 6000),
            ],
            "total": "Toplam gider",
            "income": "Aylık gelir",
            "income_value": 48000,
            "left": "Kalan",
            "format": '#,##0 "TL";[Red]-#,##0 "TL"',
        },
        "deck": {
            "title": "2026 Satış Stratejisi",
            "subtitle": "Bölge yöneticileri toplantısı · Kasım 2026",
            "points_title": "Öne çıkanlar",
            "points": [
                "Satışlar hedefin %6 üzerinde",
                "Yeni müşteri sayısı %14 arttı",
                "Çevrim içi siparişler iki katına çıktı",
                "İade oranı %2,1'e geriledi",
            ],
            "chart_title": "Çeyreklik satışlar (milyon TL)",
            "bars": [("1. Ç", 3.2), ("2. Ç", 3.7), ("3. Ç", 4.4), ("4. Ç (hedef)", 4.8)],
            "decimal": ",",
        },
        "minutes": {
            "title": "Toplantı Tutanağı",
            "subtitle": "Yeni ofis taşınma projesi",
            "facts": [
                ("Tarih:", "12 Kasım 2026, 10.00-11.30"),
                ("Yer:", "Toplantı salonu 2"),
                ("Katılımcılar:", "Proje ekibi, satın alma, bilgi işlem (7 kişi)"),
            ],
            "agenda_title": "Gündem",
            "agenda": [
                "1. Taşınma takvimi ve ara hedefler",
                "2. Mobilya ve donanım alımı",
                "3. Ağ ve telefon altyapısı",
            ],
            "decisions_title": "Kararlar",
            "decisions": [
                "Taşınma iki aşamada, 5 ve 19 Aralık hafta sonlarında yapılacak.",
                "Mobilya teklifleri 20 Kasım'a kadar toplanacak; en az üç teklif istenecek.",
                "Bilgi işlem ekibi kablolama planını bir hafta içinde paylaşacak.",
            ],
            "tasks_title": "Görevler",
            "tasks_head": ["Görev", "Sorumlu", "Son tarih"],
            "tasks": [
                ["Teklif dosyası", "Satın alma", "20.11.2026"],
                ["Kablolama planı", "Bilgi işlem", "19.11.2026"],
                ["Taşınma duyurusu", "İnsan kaynakları", "25.11.2026"],
            ],
            "footer": "Sayfa 1 / 1",
        },
        "txt": "Süt\nEkmek\nYumurta (10'lu)\nDomates, salatalık\nPeynir\nZeytinyağı\nÇay\n",
        "csv": "Ürün,Adet,Raf\nA4 kâğıt (500 yaprak),120,B-2\nKurşun kalem,340,A-1\nDosya klasörü,86,B-4\n"
               "Zımba teli,52,A-3\n",
        "md": "# Proje planı\n\n## Hedef\n\nYeni ofise taşınmayı aralık sonuna kadar tamamlamak.\n\n"
              "## Adımlar\n\n1. Teklifleri topla\n2. Kablolama planını onayla\n3. Taşınma günlerini duyur\n",
    },
    "en-US": {
        "files": {
            "docx": "Quarterly Report.docx",
            "xlsx": "Household Budget 2026.xlsx",
            "pptx": "Sales Review.pptx",
            "pdf": "Meeting Minutes.pdf",
            "txt": "Shopping List.txt",
            "csv": "Inventory.csv",
            "md": "Project Plan.md",
        },
        "report": {
            "title": "Third Quarter Activity Report",
            "subtitle": "North Region Office · October 2026",
            "sections": [
                ("Summary", [
                    "Third-quarter sales came in 6 percent above target. The number of new customers grew "
                    "by 14 percent year over year, and the return rate fell to 2.1 percent.",
                ]),
                ("Sales results", "TABLE"),
                ("Next steps", [
                    "• Finalise the fourth-quarter campaign calendar in early November.",
                    "• Complete the warehouse stock count before the end of the year.",
                    "• Share the customer satisfaction survey results with the team.",
                ]),
            ],
            "table_head": ["Product group", "Target", "Actual", "Ratio"],
            "table_rows": [
                ["Office furniture", "$120,000", "$131,500", "110%"],
                ["Stationery", "$45,000", "$43,850", "97%"],
                ["Computers and hardware", "$210,000", "$224,000", "107%"],
                ["Technical service", "$38,000", "$40,200", "106%"],
            ],
            "table_total": ["Total", "$413,000", "$439,550", "106%"],
        },
        "budget": {
            "sheet": "Budget",
            "title": "Monthly Household Budget · November 2026",
            "head": ["Item", "Planned", "Actual", "Difference"],
            "rows": [
                ("Rent", 1800, 1800),
                ("Groceries", 950, 1024),
                ("Electricity, water, gas", 320, 287),
                ("Transport", 250, 231),
                ("Internet and phone", 90, 90),
                ("Education", 400, 400),
                ("Health", 100, 65),
                ("Leisure", 200, 246),
                ("Savings", 600, 600),
            ],
            "total": "Total spending",
            "income": "Monthly income",
            "income_value": 4800,
            "left": "Left over",
            "format": '"$"#,##0;[Red]-"$"#,##0',
        },
        "deck": {
            "title": "2026 Sales Strategy",
            "subtitle": "Regional managers' meeting · November 2026",
            "points_title": "Highlights",
            "points": [
                "Sales 6% above target",
                "New customers up 14%",
                "Online orders doubled",
                "Return rate down to 2.1%",
            ],
            "chart_title": "Quarterly sales ($ million)",
            "bars": [("Q1", 3.2), ("Q2", 3.7), ("Q3", 4.4), ("Q4 (target)", 4.8)],
            "decimal": ".",
        },
        "minutes": {
            "title": "Meeting Minutes",
            "subtitle": "New office move project",
            "facts": [
                ("Date:", "12 November 2026, 10:00-11:30"),
                ("Place:", "Meeting room 2"),
                ("Attendees:", "Project team, purchasing, IT (7 people)"),
            ],
            "agenda_title": "Agenda",
            "agenda": [
                "1. Moving schedule and milestones",
                "2. Furniture and hardware purchases",
                "3. Network and phone infrastructure",
            ],
            "decisions_title": "Decisions",
            "decisions": [
                "The move takes place in two stages, on the weekends of 5 and 19 December.",
                "Furniture quotes are collected by 20 November; at least three are requested.",
                "The IT team shares the cabling plan within a week.",
            ],
            "tasks_title": "Tasks",
            "tasks_head": ["Task", "Owner", "Due"],
            "tasks": [
                ["Quote file", "Purchasing", "11/20/2026"],
                ["Cabling plan", "IT", "11/19/2026"],
                ["Moving announcement", "Human resources", "11/25/2026"],
            ],
            "footer": "Page 1 of 1",
        },
        "txt": "Milk\nBread\nEggs (dozen)\nTomatoes, cucumbers\nCheese\nOlive oil\nTea\n",
        "csv": "Item,Count,Shelf\nA4 paper (500 sheets),120,B-2\nPencils,340,A-1\nFile folders,86,B-4\n"
               "Staples,52,A-3\n",
        "md": "# Project plan\n\n## Goal\n\nComplete the move to the new office by the end of December.\n\n"
              "## Steps\n\n1. Collect quotes\n2. Approve the cabling plan\n3. Announce the moving days\n",
    },
}

# What the screenshot pass opens, in order, and what it calls each screenshot (fastlane sorts by
# name). The home screen (01-home) and the PDF's drawing tools (06-pdf-annotate) need no file of
# their own.
SHOTS = [("02-word", "docx", DOCX_MIME), ("03-sheet", "xlsx", XLSX_MIME), ("04-slides", "pptx", PPTX_MIME),
         ("05-pdf", "pdf", "application/pdf")]
LIST_ONLY = [("txt", "text/plain"), ("csv", "text/csv"), ("md", "text/markdown")]


# --------------------------------------------------------------------------------------------
# Word


def w_run(text, bold=False, color=None, size=None):
    props = ("<w:b/>" if bold else "") + (f'<w:color w:val="{color}"/>' if color else "") + (
        f'<w:sz w:val="{size}"/><w:szCs w:val="{size}"/>' if size else "")
    return (f"<w:r>{f'<w:rPr>{props}</w:rPr>' if props else ''}"
            f'<w:t xml:space="preserve">{xml_escape(text)}</w:t></w:r>')


def w_para(text, style=None, **run):
    return "<w:p>" + (f'<w:pPr><w:pStyle w:val="{style}"/></w:pPr>' if style else "") + w_run(text, **run) + "</w:p>"


def w_table(head, rows, total):
    widths = [3400, 1900, 1900, 1300]

    def cell(text, width, fill=None, bold=False, color=None, right=False):
        shade = f'<w:shd w:val="clear" w:color="auto" w:fill="{fill}"/>' if fill else ""
        align = '<w:jc w:val="right"/>' if right else ""
        return (f'<w:tc><w:tcPr><w:tcW w:w="{width}" w:type="dxa"/>{shade}</w:tcPr>'
                f'<w:p><w:pPr><w:spacing w:before="60" w:after="60"/>{align}</w:pPr>'
                f"{w_run(text, bold=bold, color=color)}</w:p></w:tc>")

    def row(values, **kw):
        return "<w:tr>" + "".join(cell(v, widths[i], right=i > 0, **kw) for i, v in enumerate(values)) + "</w:tr>"

    border = "".join(f'<w:{s} w:val="single" w:sz="4" w:space="0" w:color="9DC3E6"/>'
                     for s in ("top", "left", "bottom", "right", "insideH", "insideV"))
    return ('<w:tbl><w:tblPr><w:tblW w:w="8500" w:type="dxa"/>'
            f"<w:tblBorders>{border}</w:tblBorders></w:tblPr>"
            "<w:tblGrid>" + "".join(f'<w:gridCol w:w="{w}"/>' for w in widths) + "</w:tblGrid>"
            + row(head, fill=NAVY, bold=True, color="FFFFFF")
            + "".join(row(r, fill=LIGHT if i % 2 else None) for i, r in enumerate(rows))
            + row(total, bold=True, fill=LIGHT)
            + "</w:tbl>")


def make_report(path, c):
    body = [w_para(c["title"], "Title"), w_para(c["subtitle"], "Subtitle")]
    for heading, content in c["sections"]:
        body.append(w_para(heading, "Heading1"))
        if content == "TABLE":
            body.append(w_table(c["table_head"], c["table_rows"], c["table_total"]))
        else:
            body.extend(w_para(text) for text in content)
    w = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
    fonts = '<w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:eastAsia="Calibri" w:cs="Calibri"/>'
    styles = (
        f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles {w}>'
        f'<w:docDefaults><w:rPrDefault><w:rPr>{fonts}<w:sz w:val="22"/><w:szCs w:val="22"/>'
        f'<w:lang w:val="{c["lang"]}"/></w:rPr></w:rPrDefault>'
        '<w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="276" w:lineRule="auto"/></w:pPr></w:pPrDefault>'
        "</w:docDefaults>"
        '<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>'
        '<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/>'
        '<w:pPr><w:spacing w:after="60"/></w:pPr>'
        f'<w:rPr><w:b/><w:color w:val="{NAVY}"/><w:sz w:val="48"/><w:szCs w:val="48"/></w:rPr></w:style>'
        '<w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/><w:basedOn w:val="Normal"/>'
        '<w:pPr><w:spacing w:after="240"/></w:pPr>'
        '<w:rPr><w:color w:val="595959"/><w:sz w:val="26"/><w:szCs w:val="26"/></w:rPr></w:style>'
        '<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/>'
        '<w:pPr><w:keepNext/><w:spacing w:before="240" w:after="80"/><w:outlineLvl w:val="0"/></w:pPr>'
        f'<w:rPr><w:b/><w:color w:val="{BLUE}"/><w:sz w:val="30"/><w:szCs w:val="30"/></w:rPr></w:style>'
        "</w:styles>"
    )
    rels = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
    write_zip(path, [
        ("[Content_Types].xml",
         '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
         '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
         '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
         '<Default Extension="xml" ContentType="application/xml"/>'
         '<Override PartName="/word/document.xml" '
         'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
         '<Override PartName="/word/styles.xml" '
         'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>'),
        ("_rels/.rels",
         '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
         '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
         f'<Relationship Id="rId1" Type="{rels}/officeDocument" Target="word/document.xml"/></Relationships>'),
        ("word/_rels/document.xml.rels",
         '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
         '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
         f'<Relationship Id="rId1" Type="{rels}/styles" Target="styles.xml"/></Relationships>'),
        ("word/styles.xml", styles),
        ("word/document.xml",
         f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {w}><w:body>{"".join(body)}'
         '<w:sectPr><w:pgSz w:w="11906" w:h="16838"/>'
         '<w:pgMar w:top="1134" w:right="1134" w:bottom="1134" w:left="1134"/></w:sectPr>'
         "</w:body></w:document>"),
    ])


# --------------------------------------------------------------------------------------------
# Spreadsheet

# cellXfs, in this order: 0 default, 1 title, 2 header, 3 text, 4 money, 5 total label, 6 total money,
# 7 banded text, 8 banded money.
XLSX_STYLES = (
    '<numFmts count="1"><numFmt numFmtId="164" formatCode="{fmt}"/></numFmts>'
    '<fonts count="4"><font><sz val="11"/><name val="Calibri"/></font>'
    '<font><b/><sz val="11"/><name val="Calibri"/></font>'
    '<font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font>'
    f'<font><b/><sz val="15"/><color rgb="FF{NAVY}"/><name val="Calibri"/></font></fonts>'
    '<fills count="4"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill>'
    f'<fill><patternFill patternType="solid"><fgColor rgb="FF{NAVY}"/><bgColor indexed="64"/></patternFill></fill>'
    f'<fill><patternFill patternType="solid"><fgColor rgb="FF{LIGHT}"/><bgColor indexed="64"/></patternFill></fill>'
    "</fills>"
    '<borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border>'
    f'<border><left/><right/><top style="thin"><color rgb="FF{NAVY}"/></top>'
    f'<bottom style="thin"><color rgb="FF{NAVY}"/></bottom><diagonal/></border></borders>'
    '<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>'
    '<cellXfs count="9">'
    '<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>'
    '<xf numFmtId="0" fontId="3" fillId="0" borderId="0" xfId="0" applyFont="1"/>'
    '<xf numFmtId="0" fontId="2" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1" applyAlignment="1">'
    '<alignment horizontal="center"/></xf>'
    '<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>'
    '<xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>'
    '<xf numFmtId="0" fontId="1" fillId="3" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1"/>'
    '<xf numFmtId="164" fontId="1" fillId="3" borderId="1" xfId="0" applyNumberFormat="1" applyFont="1" '
    'applyFill="1" applyBorder="1"/>'
    '<xf numFmtId="0" fontId="0" fillId="3" borderId="0" xfId="0" applyFill="1"/>'
    '<xf numFmtId="164" fontId="0" fillId="3" borderId="0" xfId="0" applyNumberFormat="1" applyFill="1"/>'
    "</cellXfs>"
    '<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>'
)


def make_budget(path, c):
    strings = []

    def s(text):
        if text not in strings:
            strings.append(text)
        return strings.index(text)

    rows = []  # (row number, [(column letter, xml)])

    def text(col, row, value, style):
        return f'<c r="{col}{row}" s="{style}" t="s"><v>{s(value)}</v></c>'

    def num(col, row, value, style, formula=None):
        f = f"<f>{formula}</f>" if formula else ""
        return f'<c r="{col}{row}" s="{style}">{f}<v>{value}</v></c>'

    rows.append((1, [text("A", 1, c["title"], 1)]))
    rows.append((3, [text(col, 3, h, 2) for col, h in zip("ABCD", c["head"])]))
    first = 4
    planned_sum = actual_sum = 0
    for i, (item, planned, actual) in enumerate(c["rows"]):
        r = first + i
        band = i % 2 == 1
        planned_sum += planned
        actual_sum += actual
        rows.append((r, [text("A", r, item, 7 if band else 3), num("B", r, planned, 8 if band else 4),
                         num("C", r, actual, 8 if band else 4),
                         num("D", r, planned - actual, 8 if band else 4, f"B{r}-C{r}")]))
    last = first + len(c["rows"]) - 1
    t = last + 1
    rows.append((t, [text("A", t, c["total"], 5), num("B", t, planned_sum, 6, f"SUM(B{first}:B{last})"),
                     num("C", t, actual_sum, 6, f"SUM(C{first}:C{last})"),
                     num("D", t, planned_sum - actual_sum, 6, f"SUM(D{first}:D{last})")]))
    income = t + 2
    rows.append((income, [text("A", income, c["income"], 3), num("C", income, c["income_value"], 4)]))
    rows.append((income + 1, [text("A", income + 1, c["left"], 5),
                              num("C", income + 1, c["income_value"] - actual_sum, 6, f"C{income}-C{t}")]))
    sheet_rows = "".join(f'<row r="{r}">{"".join(cells)}</row>' for r, cells in rows)

    main = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
    rel = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
    ct = "application/vnd.openxmlformats-officedocument.spreadsheetml"
    rt = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
    styles = XLSX_STYLES.replace("{fmt}", xml_escape(c["format"]).replace('"', "&quot;"))
    write_zip(path, [
        ("[Content_Types].xml",
         '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
         '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
         '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
         '<Default Extension="xml" ContentType="application/xml"/>'
         f'<Override PartName="/xl/workbook.xml" ContentType="{ct}.sheet.main+xml"/>'
         f'<Override PartName="/xl/worksheets/sheet1.xml" ContentType="{ct}.worksheet+xml"/>'
         f'<Override PartName="/xl/sharedStrings.xml" ContentType="{ct}.sharedStrings+xml"/>'
         f'<Override PartName="/xl/styles.xml" ContentType="{ct}.styles+xml"/></Types>'),
        ("_rels/.rels",
         '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
         '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
         f'<Relationship Id="rId1" Type="{rt}/officeDocument" Target="xl/workbook.xml"/></Relationships>'),
        ("xl/workbook.xml",
         f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook {main} {rel}>'
         f'<sheets><sheet name="{xml_escape(c["sheet"])}" sheetId="1" r:id="rId1"/></sheets></workbook>'),
        ("xl/_rels/workbook.xml.rels",
         '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
         '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
         f'<Relationship Id="rId1" Type="{rt}/worksheet" Target="worksheets/sheet1.xml"/>'
         f'<Relationship Id="rId2" Type="{rt}/sharedStrings" Target="sharedStrings.xml"/>'
         f'<Relationship Id="rId3" Type="{rt}/styles" Target="styles.xml"/></Relationships>'),
        ("xl/worksheets/sheet1.xml",
         f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet {main}>'
         '<sheetViews><sheetView workbookViewId="0"><selection activeCell="A1" sqref="A1"/></sheetView></sheetViews>'
         '<cols><col min="1" max="1" width="26" customWidth="1"/>'
         '<col min="2" max="4" width="14" customWidth="1"/></cols>'
         f"<sheetData>{sheet_rows}</sheetData></worksheet>"),
        ("xl/sharedStrings.xml",
         f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
         f'<sst {main} count="{len(strings)}" uniqueCount="{len(strings)}">'
         + "".join(f"<si><t>{xml_escape(x)}</t></si>" for x in strings) + "</sst>"),
        ("xl/styles.xml",
         f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet {main}>{styles}</styleSheet>'),
    ])


# --------------------------------------------------------------------------------------------
# Presentation (the master, layouts and theme of the repository's python-pptx test deck)

A = 'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"'
P = 'xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"'
R = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
SLIDE_W, SLIDE_H = 9144000, 6858000
# The language of the runs a_run writes (set by make_deck for each deck).
LANG = ["tr-TR"]


def a_run(text, size=None, color=None, bold=False):
    attrs = f' lang="{LANG[0]}"' + (f' sz="{size}"' if size else "") + (' b="1"' if bold else "")
    fill = f'<a:solidFill><a:srgbClr val="{color}"/></a:solidFill>' if color else ""
    return f'<a:r><a:rPr{attrs}>{fill}</a:rPr><a:t>{xml_escape(text)}</a:t></a:r>'


def placeholder(shape_id, ph, paragraphs, frame=""):
    return (f'<p:sp><p:nvSpPr><p:cNvPr id="{shape_id}" name="Placeholder {shape_id}"/>'
            f'<p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr>{ph}</p:nvPr></p:nvSpPr>'
            f"<p:spPr>{frame}</p:spPr><p:txBody><a:bodyPr/><a:lstStyle/>{''.join(paragraphs)}</p:txBody></p:sp>")


def xfrm(x, y, w, h):
    return f'<a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></a:xfrm>'


def box(shape_id, x, y, w, h, fill=None, paragraphs=(), anchor="ctr"):
    fill_xml = f'<a:solidFill><a:srgbClr val="{fill}"/></a:solidFill>' if fill else "<a:noFill/>"
    text = "".join(paragraphs) or "<a:p/>"
    return (f'<p:sp><p:nvSpPr><p:cNvPr id="{shape_id}" name="Shape {shape_id}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>'
            f'<p:spPr>{xfrm(x, y, w, h)}<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>{fill_xml}'
            f'<a:ln><a:noFill/></a:ln></p:spPr>'
            f'<p:txBody><a:bodyPr wrap="square" lIns="0" rIns="0" tIns="0" bIns="0" anchor="{anchor}"/>'
            f"<a:lstStyle/>{text}</p:txBody></p:sp>")


def centered(text, **run):
    return f'<a:p><a:pPr algn="ctr"/>{a_run(text, **run)}</a:p>'


def slide_xml(shapes):
    return (f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld {A} {P} {R}><p:cSld><p:spTree>'
            '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>'
            f'{"".join(shapes)}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>')


def make_deck(path, c):
    LANG[0] = c["lang"]
    source = REPO / "rust/mo-deck/testdata/grouped-text.pptx"
    with zipfile.ZipFile(source) as z:
        parts = {info.filename: z.read(info.filename) for info in z.infolist()}
    # No thumbnail of the old slide and none of the template's author metadata.
    parts.pop("docProps/thumbnail.jpeg", None)
    parts["_rels/.rels"] = re.sub(rb'<Relationship Id="rId2"[^>]*thumbnail"[^>]*/>', b"", parts["_rels/.rels"])
    if b"thumbnail" in parts["_rels/.rels"]:
        sys.exit(f"{source}: unexpected package relationships")
    parts["docProps/core.xml"] = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" '
        'xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>' + xml_escape(c["title"]) + "</dc:title>"
        "</cp:coreProperties>").encode("utf-8")

    # 1: title slide on a dark band.
    title = slide_xml([
        box(10, 0, 0, SLIDE_W, SLIDE_H, fill=NAVY),
        box(11, 0, 4400000, SLIDE_W, 80000, fill="F4B183"),
        placeholder(2, '<p:ph type="ctrTitle"/>', [centered(c["title"], size=4400, color="FFFFFF", bold=True)],
                    xfrm(685800, 2130425, 7772400, 1470025)),
        placeholder(3, '<p:ph type="subTitle" idx="1"/>', [centered(c["subtitle"], size=2000, color="DDEBF7")],
                    xfrm(1371600, 3657600, 6400800, 600000)),
    ])
    # 2: bullet points.
    points = slide_xml([
        placeholder(2, '<p:ph type="title"/>', [f"<a:p>{a_run(c['points_title'], color=NAVY, bold=True)}</a:p>"]),
        placeholder(3, '<p:ph idx="1"/>', [f"<a:p>{a_run(p, size=2800)}</a:p>" for p in c["points"]]),
    ])
    # 3: a bar chart drawn with rectangles (no chart part: what every reader shows the same).
    bars = c["bars"]
    top = max(v for _, v in bars)
    base_y, chart_h, left, gap, bar_w = 5800000, 3600000, 1100000, 520000, 1300000
    shapes = [placeholder(2, '<p:ph type="title"/>', [f"<a:p>{a_run(c['chart_title'], color=NAVY, bold=True)}</a:p>"]),
              box(20, left - 200000, base_y, 4 * bar_w + 3 * gap + 400000, 20000, fill="7F7F7F")]
    for i, (label, value) in enumerate(bars):
        h = int(chart_h * value / top)
        x = left + i * (bar_w + gap)
        color = "F4B183" if i == len(bars) - 1 else BLUE
        shown = f"{value:.1f}".replace(".", c["decimal"])
        shapes.append(box(30 + i, x, base_y - h, bar_w, h, fill=color))
        shapes.append(box(40 + i, x - 150000, base_y - h - 420000, bar_w + 300000, 380000,
                          paragraphs=[centered(shown, size=2000, bold=True, color="404040")]))
        shapes.append(box(50 + i, x - 250000, base_y + 80000, bar_w + 500000, 380000,
                          paragraphs=[centered(label, size=1600, color="404040")], anchor="t"))
    chart = slide_xml(shapes)

    layouts = {1: "slideLayout1.xml", 2: "slideLayout2.xml", 3: "slideLayout6.xml"}
    slide_type = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml"
    rel_type = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
    types = parts["[Content_Types].xml"].decode("utf-8")
    rels = parts["ppt/_rels/presentation.xml.rels"].decode("utf-8")
    presentation = parts["ppt/presentation.xml"].decode("utf-8")
    ids = []
    for number, xml in enumerate([title, points, chart], start=1):
        parts[f"ppt/slides/slide{number}.xml"] = xml.encode("utf-8")
        parts[f"ppt/slides/_rels/slide{number}.xml.rels"] = (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            f'<Relationship Id="rId1" Type="{rel_type}/slideLayout" Target="../slideLayouts/{layouts[number]}"/>'
            "</Relationships>").encode("utf-8")
        if number > 1:
            types = types.replace(
                "</Types>", f'<Override PartName="/ppt/slides/slide{number}.xml" ContentType="{slide_type}"/></Types>')
            rels = rels.replace("</Relationships>", f'<Relationship Id="rIdS{number}" Type="{rel_type}/slide" '
                                f'Target="slides/slide{number}.xml"/></Relationships>')
            ids.append(f'<p:sldId id="{255 + number}" r:id="rIdS{number}"/>')
        else:
            first = re.search(r'Id="(rId\d+)"[^>]*Target="slides/slide1\.xml"', rels)
            if not first:
                sys.exit(f"{source} no longer has the one-slide layout make_deck expects")
            ids.append(f'<p:sldId id="256" r:id="{first.group(1)}"/>')
    presentation, count = re.subn(r"<p:sldIdLst>.*?</p:sldIdLst>", f"<p:sldIdLst>{''.join(ids)}</p:sldIdLst>",
                                  presentation)
    if count != 1:
        sys.exit(f"{source} has no slide list")
    parts["[Content_Types].xml"] = types.encode("utf-8")
    parts["ppt/_rels/presentation.xml.rels"] = rels.encode("utf-8")
    parts["ppt/presentation.xml"] = presentation.encode("utf-8")
    write_zip(path, sorted(parts.items(), key=lambda item: item[0] != "[Content_Types].xml"))


# --------------------------------------------------------------------------------------------
# PDF (standard Helvetica; the Turkish letters WinAnsi lacks get codes 128-133 by glyph name)

TURKISH_GLYPHS = {"Ğ": 128, "ğ": 129, "İ": 130, "ı": 131, "Ş": 132, "ş": 133}
ENCODING = b"<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [128 /Gbreve /gbreve /Idotaccent " \
           b"/dotlessi /Scedilla /scedilla] >>"


def pdf_text(text):
    out = bytearray()
    for ch in unicodedata.normalize("NFC", text):
        if ch in TURKISH_GLYPHS:
            out.append(TURKISH_GLYPHS[ch])
        else:
            out += ch.encode("cp1252")
    return bytes(out).replace(b"\\", b"\\\\").replace(b"(", b"\\(").replace(b")", b"\\)")


def make_minutes(path, c):
    ops = []

    def text(x, y, value, size=11, bold=False, gray=0.15, white=False):
        color = b"1 1 1 rg" if white else f"{gray} {gray} {gray} rg".encode()
        font = b"/F2" if bold else b"/F1"
        ops.append(color + b" BT " + font + f" {size} Tf {x} {y} Td (".encode() + pdf_text(value) + b") Tj ET")

    def rect(x, y, w, h, rgb):
        ops.append(f"{rgb} rg {x} {y} {w} {h} re f".encode())

    rect(0, 742, 595, 100, "0.122 0.306 0.475")
    rect(0, 738, 595, 4, "0.957 0.694 0.514")
    text(56, 792, c["title"], size=26, bold=True, white=True)
    text(56, 764, c["subtitle"], size=13, white=True)
    y = 700
    for label, value in c["facts"]:
        text(56, y, label, bold=True)
        text(150, y, value)
        y -= 20
    y -= 18
    for heading, items in ((c["agenda_title"], c["agenda"]), (c["decisions_title"], c["decisions"])):
        text(56, y, heading, size=15, bold=True, gray=0.18)
        ops.append(f"0.18 0.459 0.714 RG 1 w 56 {y - 6} m 539 {y - 6} l S".encode())
        y -= 28
        for item in items:
            text(66, y, item if item[0].isdigit() else "•  " + item)
            y -= 19
        y -= 16
    text(56, y, c["tasks_title"], size=15, bold=True, gray=0.18)
    y -= 14
    cols = [56, 270, 430, 539]
    row_h = 24
    rows = [c["tasks_head"], *c["tasks"]]
    for i, row in enumerate(rows):
        top = y - i * row_h
        if i == 0:
            rect(56, top - row_h, 483, row_h, "0.867 0.922 0.969")
        for j, value in enumerate(row):
            text(cols[j] + 8, top - 16, value, bold=i == 0)
    bottom = y - len(rows) * row_h
    lines = [f"{cols[0]} {y - i * row_h} m {cols[-1]} {y - i * row_h} l" for i in range(len(rows) + 1)]
    lines += [f"{x} {y} m {x} {bottom} l" for x in cols]
    ops.append(("0.6 0.6 0.6 RG 0.75 w " + " ".join(lines) + " S").encode())
    text(56, 40, c["footer"], size=9, gray=0.45)

    content = b"\n".join(ops)
    write_pdf(path, [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> "
        b"/Contents 7 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding 6 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding 6 0 R >>",
        ENCODING,
        b"<< /Length " + str(len(content)).encode() + b" >>\nstream\n" + content + b"\nendstream",
    ])


# --------------------------------------------------------------------------------------------


def main(out):
    out.mkdir(parents=True, exist_ok=True)
    rows = []
    for locale, c in CONTENT.items():
        folder = out / locale
        folder.mkdir(exist_ok=True)
        c["report"]["lang"] = c["deck"]["lang"] = locale
        names = {kind: unicodedata.normalize("NFC", name) for kind, name in c["files"].items()}
        make_report(folder / names["docx"], c["report"])
        make_budget(folder / names["xlsx"], c["budget"])
        make_deck(folder / names["pptx"], c["deck"])
        make_minutes(folder / names["pdf"], c["minutes"])
        for kind, _ in LIST_ONLY:
            (folder / names[kind]).write_text(c[kind], encoding="utf-8")
        rows += [(locale, shot, names[kind], mime) for shot, kind, mime in SHOTS]
        rows += [(locale, "-", names[kind], mime) for kind, mime in LIST_ONLY]
    (out / "shots.tsv").write_text("".join("\t".join(r) + "\n" for r in rows), encoding="utf-8")
    for locale, shot, name, mime in rows:
        print(f"{locale:6} {shot:10} {(out / locale / name).stat().st_size:>7} B  {name:28} {mime}")


if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1].startswith("-"):
        sys.exit(f"usage: {sys.argv[0]} OUTPUT_DIR")
    main(pathlib.Path(sys.argv[1]))
