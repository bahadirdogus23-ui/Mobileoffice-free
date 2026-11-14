//! A fallback font for letters the engine's built-in drafting font lacks.
//!
//! The engine draws SHX-font text (and text whose TrueType font isn't installed) in its own
//! single-stroke font, which has ASCII and a few symbols only; any other character (İ ı Ş ş Ğ ğ
//! Ç ç Ö ö Ü ü, and every other non-ASCII letter) is drawn from an installed font that has it,
//! else as an empty box. It looks for installed fonts in the desktop systems' font folders
//! (`cadcraft_fonts::ttf`, `scan`), never in Android's `/system/fonts`, so on a phone all those
//! letters were boxes. Opening the first drawing registers one of Android's own Latin fonts as
//! the fallback.

use std::path::Path;
use std::sync::Arc;

/// Where Android keeps its fonts.
pub(crate) const SYSTEM_FONTS: &str = "/system/fonts";

/// Android's Latin, Greek and Cyrillic sans-serif fonts, best first: Roboto (a static file
/// since Android 13 alongside the variable one), then the fonts of older and other systems.
const CANDIDATES: [&str; 5] = ["RobotoStatic-Regular.ttf", "Roboto-Regular.ttf", "NotoSans-Regular.ttf", "DroidSans.ttf", "DejaVuSans.ttf"];

/// Font files larger than this are not candidates (Roboto is well under it).
const MAX_BYTES: u64 = 8 << 20;

/// Letters a fallback must have to be chosen: the Turkish ones, which the drafting font lacks.
const MUST_HAVE: [char; 12] = ['İ', 'ı', 'Ş', 'ş', 'Ğ', 'ğ', 'Ç', 'ç', 'Ö', 'ö', 'Ü', 'ü'];

/// Registers the first font in `dir` from [`CANDIDATES`] that has the Turkish letters as the
/// engine's fallback font; returns its file name, or `None` when `dir` has none.
pub(crate) fn register_fallback(dir: &Path) -> Option<&'static str> {
    for name in CANDIDATES {
        let path = dir.join(name);
        if std::fs::metadata(&path).map_or(true, |m| !m.is_file() || m.len() > MAX_BYTES) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let bytes = Arc::new(bytes);
        if !cadcraft_fonts::ttf::faces(&bytes).first().is_some_and(|f| MUST_HAVE.iter().all(|&c| f.has_glyph(c))) {
            continue;
        }
        cadcraft_fonts::ttf::register(name, Arc::unwrap_or_clone(bytes));
        return Some(name);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty folder under the system temporary folder.
    fn folder(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("mo-cad-fonts-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_file_that_is_no_font_is_skipped() {
        let d = folder("bad");
        std::fs::write(d.join("Roboto-Regular.ttf"), b"not a font").unwrap();
        assert_eq!(register_fallback(&d), None);
        assert_eq!(register_fallback(&d.join("missing")), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// With a font that has the Turkish letters (DejaVu Sans, as most Linux systems have it,
    /// standing in for Android's Roboto) under a name Android uses, it becomes the fallback.
    #[test]
    fn a_system_font_with_turkish_letters_becomes_the_fallback() {
        let host =
            ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/dejavu/DejaVuSans.ttf"];
        let Some(src) = host.iter().map(Path::new).find(|p| p.is_file()) else {
            eprintln!("no DejaVu Sans on this system; skipped");
            return;
        };
        let d = folder("good");
        std::fs::copy(src, d.join("Roboto-Regular.ttf")).unwrap();
        assert_eq!(register_fallback(&d), Some("Roboto-Regular.ttf"));
        let face = cadcraft_fonts::ttf::find("Roboto-Regular").unwrap();
        assert!(face.has_glyph('ı') && face.has_glyph('İ'));
        assert!(cadcraft_fonts::ttf::fallback_for('ı').is_some());
        let _ = std::fs::remove_dir_all(&d);
    }
}
