# Local patch to GridCraft's PDF writer

Upstream: `storytold/gridcraft`, crate `crates/pdf`, commit `c0ee120ac2d9b59630f5abf1151dd9b471fe2926`
(MIT OR Apache-2.0; license files alongside). `rust/Cargo.toml` puts this copy in place of the git
crate with `[patch]`.

Rebased from `11bb673` onto `c0ee120` with the GridCraft bump (the engine there calls the new
`Point`/`Rect`/`Font::Symbol` API, see `README.md`). The upstream files were copied as they are and
the three changes below applied again; the upstream tests run unchanged apart from the expected
font encoding. Upstream's new `Font::Symbol` keeps its own encoding and is never replaced by an
embedded face (`set_text_fonts` still takes the 12 text slots).

## 1. Turkish letters in the standard fonts

**Change:** Turkish letters in PDFs exported from spreadsheets.

- **Upstream behaviour:** Standard-font text is written in WinAnsiEncoding (CP1252), which lacks Ğ ğ İ ı Ş ş. These letters go to a non-embedded Chinese CID font (STSong-Light). Phones and most PDF viewers do not have that font, so the letters vanish.
- **This patch:** The fonts get a `/Differences` encoding that puts those six glyphs at the codes Windows-1254 uses (0xD0, 0xDD, 0xDE, 0xF0, 0xFD, 0xFE). Every standard Latin font has these glyphs. The Latin-1 letters normally at those codes (Ð Ý Þ ð ý þ) move to the CID path.
- **Status:** The patch is to be offered upstream. When upstream has an equivalent fix, delete this folder and the `[patch]` entry.

## 2. Embedded TrueType faces (`src/fonts.rs`)

- **Problem:** Some viewers do not have Turkish İ (`/Idotaccent`) in the fonts they use for the non-embedded standard fonts. Android's PdfRenderer is one of them, so İ still vanished after patch 1.
- **Change:** `set_text_fonts` takes TrueType faces for the 12 standard slots. Every PDF written after that call embeds them as simple TrueType fonts:
  - The encoding is the same as in patch 1.
  - Widths come from the face, both in the PDF and in `text_width`.
  - The font file is compressed with zlib.
- **In this app:** `mo-grid` registers WordCraft's bundled OFL fonts (Source Sans 3, Source Serif 4 and JetBrains Mono). These fonts are already in the app, so the APK does not grow.
- **Verified:** A rendered page shows İ Ğ Ş ı ğ ş correctly.

## 3. `code_char` is public

- **Change:** `code_char` (the character a code stands for in the Turkish text encoding) is `pub`.
- **Why:** `mo-pdf` embeds the same font and encoding into PDF forms it fills, and needs the
  same code-to-character table to compute the font's widths.
