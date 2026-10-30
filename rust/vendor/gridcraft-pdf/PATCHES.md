# Local patch to GridCraft's PDF writer

Upstream: `storytold/gridcraft`, crate `crates/pdf`, commit `11bb67386c9bb19b0717e21e98adc608162ae2e5`
(MIT OR Apache-2.0; license files alongside). `rust/Cargo.toml` puts this copy in place of the git
crate with `[patch]`.

**Change:** Turkish letters in PDFs exported from spreadsheets.

- **Upstream behaviour:** Standard-font text is written in WinAnsiEncoding (CP1252), which lacks Ğ ğ İ ı Ş ş. These letters go to a non-embedded Chinese CID font (STSong-Light). Phones and most PDF viewers do not have that font, so the letters vanish.
- **This patch:** The fonts get a `/Differences` encoding that puts those six glyphs at the codes Windows-1254 uses (0xD0, 0xDD, 0xDE, 0xF0, 0xFD, 0xFE). Every standard Latin font has these glyphs. The Latin-1 letters normally at those codes (Ð Ý Þ ð ý þ) move to the CID path.
- **Status:** The patch is to be offered upstream. When upstream has an equivalent fix, delete this folder and the `[patch]` entry.
