#!/usr/bin/env bash
# Smoke test of the minified release APK on a running emulator or device (CI: the
# "emulator-smoke" job). Installs the APK, starts the app, then opens one sample of each format
# through a VIEW intent, the way a file manager's "Open with" does, and fails on any crash.
#
#   scripts/emulator-smoke.sh APP.apk SAMPLES_DIR OUT_DIR
#
# SAMPLES_DIR comes from scripts/make_smoke_samples.py (its samples.tsv says how to open each
# file). OUT_DIR receives logcat, a screenshot and the screen's UI tree for every step.
#
# How a document reaches the app: the samples are pushed to a hidden folder in Download and
# handed over as content://org.mobileofficefree.app.files/storage/... URIs. That authority is the
# app's own FileProvider (external-path "."), so no URI grant from the shell is involved; the app
# reads the file under "All files access" (ADR-0002), which is granted with appops the way a
# person grants it in Settings. The hidden folder keeps the samples off the home screen's list.
#
# A step passes when the document's screen is up (its "Back" button and the file name in the
# title bar are in the UI tree), the app process is still alive a few seconds later, the screen
# shows no "could not open" message, and logcat has no crash, ANR, JNI error or missing
# class/method/native library for the app's process. R8 problems often surface as
# NoClassDefFoundError inside runCatching, which the app shows as "could not open"; that is why the
# screen is checked, not just logcat. Two samples get one more step: the picture's text recognition
# (ocr-png) and, for the PDF in a non-embedded Helvetica (mo-smoke-pdf-base14.pdf), whether its
# Ğ ğ İ Ş ş are drawn, counted in the screenshot by colour (pdf-base14-letters).
#
# It is also the performance baseline (plan phase 5). For the home screen and every sample it
# records, in the summary table and in metrics.tsv:
#   - "am start": the TotalTime that `am start -W` reports, the cold start until the activity's
#     first frame (the process is force-stopped before every step);
#   - "Visible": from just before the intent is sent until the UI poll sees the document screen
#     (or its import notes, which only appear over an opened document). The poll cannot see the
#     moment itself, so it is a range: after the start of the last dump that did not show the
#     document, by the end of the dump that did (one poll, about 1-3 s on the emulator);
#   - "PSS": the app's TOTAL PSS from `dumpsys meminfo` after the document has settled.
# Slow opens and large memory use produce GitHub ::warning annotations (soft budgets), never a
# failure. The launch is measured twice: the first start after installing, then a second cold start.
#
# Second pass: afterwards the app is run the way most of its users run it, in Turkish (tr-TR), with
# the font scale at 2.0 and the dark theme, on the formats in SAMPLES_DIR/samples-tr.tsv: a subset
# of the first pass plus copies named like "RAPOR-İŞ.DOCX", "ARŞİV.ZIP" or "GİDER LİSTESİ.CSV"
# (upper-case extensions, Turkish letters, a space; percent-encoded in the content:// URI as the
# FileProvider encodes them). Under tr-TR a locale-sensitive lower-casing turns "ZIP" into "zıp"
# and breaks routing by extension (AGENTS.md invariant 6). The language is the app's own locale
# (`cmd locale set-app-locales`, Android 13+; the app declares tr and en in its localeConfig), so
# the system and the shell stay in English; the document screen must say "Geri" to pass. The
# title may be cut with "…" at that font size: a long enough prefix of the name is accepted. The
# pass has its own section in the summary and metrics-tr.tsv; the font scale, night mode and the
# app's locale are put back afterwards (also when the script stops early).
#
# Third pass, "Kaydetme" (saving; AGENTS.md invariant 4): back in English, the files in
# SAMPLES_DIR/samples-save.tsv are edited through the UI and saved over, and the file is then
# pulled with adb and checked on the host. Each case is a cold start (open_document), then:
#   - save-docx: "Edit", `input text` at the caret (the start of the document), Ctrl+S
#     (`input keycombination`); if the file did not change, Back out of editing and the top bar's
#     "Save". The pulled file must differ, be a valid ZIP and hold the typed word and the old text.
#   - save-xlsx: the formula bar's field (A1 is selected), its text deleted, a number typed, Enter;
#     then the top bar's "Save" (shown after any change: `hasUnsaved`) and the "Saved"
#     snackbar. The pulled file must differ, be a valid ZIP and hold the number and the old text.
#   - save-pptx (three slides): "Edit", the slide bar's "Text box", the word typed into its dialog,
#     "Apply" (a new text box on the first slide), then the top bar's "Save". The pulled file must
#     differ, be a valid ZIP with as many slides as before, hold the typed word on one slide and
#     every old text (taking the typed word away gives the old texts back).
#   - save-txt-utf8, save-txt-1254: "Edit", a tap in the text, a word typed, "Save". Removing the
#     typed word from the pulled file must give back the original bytes: the same encoding
#     (Windows-1254 letters stay single bytes), no byte order mark added, CRLF kept.
#   - block-xlsx-slicer, block-xlsx-vba (the engine cannot write a slicer / a VBA project in an
#     .xlsx back): the same cell edit, then the top bar's "Save" must come up (every editor shows it
#     after a change) and tapping it must open the system's "Save as" (after a one-line reason in a
#     snackbar) instead of writing; it is cancelled. Ctrl+S must do the same, and the file keeps its
#     bytes. A missing "Save" fails only after save-xlsx showed that the same edit brings it up.
#   - block-docx-damaged (a comments part cut short: mo-word's `Loss::Damaged`), block-pptx-slide (a
#     slide whose XML does not parse: DeckCraft leaves it out): the same edit as save-docx /
#     save-pptx, then Ctrl+S (and, if nothing came up, the top bar's "Save", which Word shows after
#     any change, also while editing) must open "Save as" instead of saving over the file; it is cancelled and the
#     file keeps its bytes. Without "Save as" coming up an unchanged file proves nothing: SKIP.
#   - block-txt-mixed (CRLF and LF mixed): edit and "Save"; the viewer does not write over the file
#     (a toast says why) but opens the system's "Save as", which is cancelled; the file keeps its bytes.
#   - save-pdf (a copy of rust/mo-pdf/testdata/jspdf-form.pdf): a PDF is never saved over
#     (docs/KAYIP-DENETIMI.md). The tool bar's "Fill in form" (the bar is swiped sideways until it
#     shows), the word typed into the form's first field (a text field), "Apply", then the top
#     bar's "Save", which must open the system's "Save as" with "<name>-duzenlendi.pdf" suggested;
#     its save button is tapped in the folder it opens on (Downloads on a fresh device). The opened
#     file must keep its bytes, nothing else may appear next to it, and the new file (found by name
#     anywhere in shared storage, pulled) must be a whole PDF ("%PDF-", "%%EOF") whose field holds
#     the word, with the form's fields and pages (and, when pdftotext is installed, the word and
#     the old text on the page). Without "Save as" or its save button the case is a SKIP that still
#     checks the opened file's bytes.
#   - block-pdf-nopages (a page tree that lists no page; mo-pdf's `PdfError::NoPages`): not opened
#     first; the cold start with it must show "Could not open the file: no page of this PDF could
#     be read…" (`pdf_no_pages`), not a document screen, keep its bytes and add nothing to its
#     folder.
# When a Word, slides or PDF case fails, the app's own "BadoSave" log lines for the file (save
# requested, refused and why, written, failed; for PDF also an opening refused) are added to its
# detail, as "BadoText" for text.
# Toasts (Word, text) are not in uiautomator's tree, so their messages are not checked; snackbars
# (sheets) are. A UI path that cannot be found (no Edit button, no text field…) is a SKIP with the
# reason and a warning, not a failure; a file that changed when it must not, or a valid save path
# that left the file unchanged or broken, is a failure. Its own summary section and
# metrics-save.tsv (for the opens).
#
# Fourth pass, "Kurtarma" (recovery of unsaved edits; AGENTS.md invariant 4): the files in
# SAMPLES_DIR/samples-recovery.tsv are edited and NOT saved. Once the app's recovery copy is
# written (RecoveryHub copies unsaved edits about 5 s after the first one; the app logs
# "copy written" with the tag "BadoRecovery"; at least SMOKE_RECOVERY_WAIT s are waited), the app
# is ended with `am force-stop`, which runs no onStop and so is as close as adb gets to a crash or
# the system ending it in the background. The cold start with the same file must then offer the
# copy ("Unsaved changes found": "Restore" / "Discard"). Each case is a cold start, then:
#   - recover-xlsx: the save pass's cell edit (A1 becomes TYPED_NUMBER; the top bar's "Save" must
#     come up, which shows the edit was taken), then "Restore": the formula bar (A1 is selected
#     again) must show the typed number.
#   - recover-docx: the save pass's Word edit, then Back out of editing until the top bar shows
#     "Save" (the document is changed); after "Restore" the top bar must show "Save" again (only a
#     changed document has it; the opened file is not changed) and the viewer's own search
#     ("Search", the typed word) must find the word: the page itself is a picture uiautomator
#     cannot read. Without the search field only the prompt, "Restore" and "Save" are checked.
#   - discard-xlsx: the same cell edit, then "Discard": the formula bar shows A1's old text and
#     there is no "Save"; a third cold start must not offer anything (the copy was deleted).
# In every case the opened file must keep its bytes (nothing was saved; recovery never writes
# over the file by itself) and nothing may appear next to it. On a failure the app's
# "BadoRecovery" log lines for the file are added to the detail. A UI path that cannot be found
# is a SKIP, as in the save pass. Its own summary section.
#
# Last pass, "Mağaza ekran görüntüleri" (Google Play phone screenshots; cosmetic, never fails the
# run, a problem is a warning): with SAMPLES_DIR/store from scripts/make_store_samples.py, the
# display is set to 1080x1920 at 420 dpi (9:16, as Play recommends), the font scale to 1.0, the light
# theme, and the status bar to System UI's demo mode (12:00, full battery, no notifications). The
# earlier passes' files are deleted from shared storage and from MediaStore's index (the home
# screen lists what MediaStore knows, hidden folders included); 01-home is a SKIP if it still
# shows one. For
# each store language (tr-TR, en-US) the app's data is cleared (no recent files from the earlier
# passes) and its locale set, that language's files are put in Documents (and the earlier
# language's taken away), and `screencap` saves, in OUT_DIR/play-screenshots/<locale>/: 02-word,
# 03-sheet, 04-slides, 05-pdf (each a cold start with a VIEW intent, once the document screen is
# up), 06-pdf-annotate (the PDF's "Write & draw" tool, if it can be reached) and then 01-home (the
# home screen listing those files). The names follow fastlane's phoneScreenshots order. Demo mode,
# the display size and density and the app's locale are put back afterwards. Its notes go to
# OUT_DIR/play-screenshots.tsv and its own summary section; its steps stop at SMOKE_SHOTS_TIME_LIMIT.
#
# Environment: SMOKE_OPEN_TIMEOUT (s, default 60) and SMOKE_BIG_OPEN_TIMEOUT (s, default 180, for
# the samples marked "big" in samples.tsv), SMOKE_OPEN_BUDGET (s, default 15) and
# SMOKE_BIG_OPEN_BUDGET (s, default 45), SMOKE_PSS_BUDGET_MB (default 768) (budgets are whole
# numbers), SMOKE_POLL (s between UI polls, default 1), SMOKE_SETTLE (s, default 6), SMOKE_OCR (1/0, default 1: run text recognition
# on the picture), SMOKE_LOG_IGNORE (extended regex of logcat lines to disregard, for a known
# harmless message), SMOKE_SECOND_PASS (1/0, default 1), SMOKE_TR_LOCALE (default tr-TR),
# SMOKE_TR_FONT_SCALE (default 2.0), SMOKE_TIME_LIMIT (min since the script started, default 34:
# second-pass steps that would start later are skipped, so the CI job stays within its timeout),
# SMOKE_SAVE_PASS (1/0, default 1), SMOKE_SAVE_TIME_LIMIT (min since the script started, default
# SMOKE_TIME_LIMIT + 4: save cases that would start later are skipped, and the whole pass when
# fewer than 4 min are left), SMOKE_RECOVERY_PASS (1/0, default 1), SMOKE_RECOVERY_TIME_LIMIT (min
# since the script started, default SMOKE_SAVE_TIME_LIMIT + 2: recovery cases that would start later
# are skipped, and the whole pass when fewer than 3 min are left), SMOKE_RECOVERY_WAIT (s, default 7:
# the least time between the edit and stopping the app), SMOKE_SHOTS_PASS (1/0, default 1),
# SMOKE_SHOTS_TIME_LIMIT (min since the script started, default SMOKE_RECOVERY_TIME_LIMIT + 4: the
# whole pass is skipped when fewer than 3 min are left, later screenshots when it is reached),
# SMOKE_SHOTS_LOCALES (default "tr-TR en-US"), SMOKE_SHOTS_SIZE (default 1080x1920),
# SMOKE_SHOTS_DENSITY (default 420), SMOKE_SHOTS_SETTLE (s after a document is up, default 3),
# SMOKE_SHOTS_OPEN_TIMEOUT (s, default 30).

set -euo pipefail

if [ $# -ne 3 ]; then
    echo "usage: $0 APP.apk SAMPLES_DIR OUT_DIR" >&2
    exit 2
fi
APK=$1
SAMPLES=$2
OUT=$3

PKG=org.mobileofficefree.app
ACTIVITY=$PKG/.MainActivity
DEVICE_DIR=/sdcard/Download/.mo-smoke
URI_BASE=content://$PKG.files/storage/Download/.mo-smoke
OPEN_TIMEOUT=${SMOKE_OPEN_TIMEOUT:-60}
BIG_OPEN_TIMEOUT=${SMOKE_BIG_OPEN_TIMEOUT:-180}
OPEN_BUDGET=${SMOKE_OPEN_BUDGET:-15}
BIG_OPEN_BUDGET=${SMOKE_BIG_OPEN_BUDGET:-45}
PSS_BUDGET_MB=${SMOKE_PSS_BUDGET_MB:-768}
POLL=${SMOKE_POLL:-1}
SETTLE=${SMOKE_SETTLE:-6}
OCR=${SMOKE_OCR:-1}
LOG_IGNORE=${SMOKE_LOG_IGNORE:-}
SECOND_PASS=${SMOKE_SECOND_PASS:-1}
TR_LOCALE=${SMOKE_TR_LOCALE:-tr-TR}
TR_FONT_SCALE=${SMOKE_TR_FONT_SCALE:-2.0}
TIME_LIMIT=${SMOKE_TIME_LIMIT:-34}
SAVE_PASS=${SMOKE_SAVE_PASS:-1}
SAVE_TIME_LIMIT=${SMOKE_SAVE_TIME_LIMIT:-$((TIME_LIMIT + 4))}
RECOVERY_PASS=${SMOKE_RECOVERY_PASS:-1}
RECOVERY_TIME_LIMIT=${SMOKE_RECOVERY_TIME_LIMIT:-$((SAVE_TIME_LIMIT + 2))}
RECOVERY_WAIT=${SMOKE_RECOVERY_WAIT:-7}
SHOTS_PASS=${SMOKE_SHOTS_PASS:-1}
SHOTS_TIME_LIMIT=${SMOKE_SHOTS_TIME_LIMIT:-$((RECOVERY_TIME_LIMIT + 4))}
SHOTS_LOCALES=${SMOKE_SHOTS_LOCALES:-tr-TR en-US}
SHOTS_SIZE=${SMOKE_SHOTS_SIZE:-1080x1920}
SHOTS_DENSITY=${SMOKE_SHOTS_DENSITY:-420}
SHOTS_SETTLE=${SMOKE_SHOTS_SETTLE:-3}
SHOTS_OPEN_TIMEOUT=${SMOKE_SHOTS_OPEN_TIMEOUT:-30}

# What the app says on screen (English and Turkish, android/app/src/main/res/values*/strings.xml).
FAILURE_TEXTS=(
    "Could not open the file" "Dosya açılamadı"                                 # open_failed
    "This file type is not supported yet" "Bu dosya türü henüz desteklenmiyor"  # unsupported_file
    "The app closed unexpectedly" "Uygulama beklenmedik şekilde kapandı"        # crash_title
)
NOTICE_TEXTS=("opens in an upcoming version" "gelecek sürümde açılacak")       # coming_soon
BACK_LABELS=("Back" "Geri")                                                     # back
IMPORT_NOTES=("Import notes" "İçe aktarma notları")                             # import_notes
OCR_LABELS=("Recognise text" "Metni tanı")                                      # ocr
OCR_WORKING=("Reading the text on the phone" "Metin telefonda okunuyor")        # ocr_working
OCR_DONE=("No text was found" "Metin bulunamadı" "Recognised on the phone" "Telefonda tanındı")
# Home screen texts that tell which language the app shows (settings, home_title, search_files,
# recently_opened).
HOME_TR=("Ayarlar" "Belgeler" "Dosya adında ara" "Son açılanlar")
HOME_EN=("Settings" "Documents" "Search file names" "Recently opened")
# The save pass (save, saved, edit, tool_keyboard, save_failed, save_over_incomplete, save_over_truncated).
SAVE_LABELS=("Save" "Kaydet")
# The unsaved-changes question every editor asks on leaving (UnsavedChangesDialog: save_as,
# leave_discard, leave_stay; "Save" is SAVE_LABELS). Matched exactly, so "Save as…" is not "Save".
SAVE_AS_LABELS=("Save as…" "Farklı kaydet…")
LEAVE_DISCARD_LABELS=("Exit without saving" "Kaydetmeden çık")
LEAVE_STAY_LABELS=("Keep editing" "Vazgeç")
SAVED_TEXTS=("Saved" "Kaydedildi")
EDIT_LABELS=("Edit" "Düzenle")
KEYBOARD_LABELS=("Keyboard" "Klavye")
SAVE_PROBLEM_TEXTS=("Could not save" "Kaydedilemedi" "Not saved:" "Kaydedilmedi:")
# The slide editing bar's new text box and its dialog's button (slide_add_text, apply).
TEXT_BOX_LABELS=("Text box" "Metin kutusu")
APPLY_LABELS=("Apply" "Uygula")
# The PDF screen's form tool, two tools that are always on screen at the start of its tool bar
# (to find the bar to swipe), and the reason a PDF without a readable page does not open
# (form_fill; pages, reading; pdf_no_pages).
FORM_FILL_LABELS=("Fill in form" "Formu doldur")
TOOLBAR_LABELS=("Pages" "Sayfalar" "Reading" "Okuma")
NO_PAGES_TEXTS=("no page of this PDF could be read" "hiçbir sayfası okunamadı")
# The app's logcat tag for Word, slide and PDF saves (SAVE_LOG_TAG in PagedViewModel.kt).
SAVE_LOG_TAG=BadoSave
# The recovery pass: the prompt's title and buttons (recovery_title, recovery_restore,
# recovery_discard), the viewer's search and its "no match" (search, no_matches), and the app's
# logcat tag for recovery copies (RECOVERY_LOG_TAG in Recovery.kt).
RECOVERY_TITLES=("Unsaved changes found" "Kaydedilmemiş değişiklikler bulundu")
RESTORE_LABELS=("Restore" "Geri yükle")
DISCARD_LABELS=("Discard" "Sil")
SEARCH_LABELS=("Search" "Ara")
NO_MATCH_TEXTS=("None" "Yok")
RECOVERY_LOG_TAG=BadoRecovery
# The PDF screen's drawing tool (annotate), for the last store screenshot.
ANNOTATE_LABELS=("Write & draw" "Yaz ve çiz")
# What is typed: ASCII, as `input text` sends it; the cell gets a number.
TYPED_WORD=Smoke42
TYPED_NUMBER=4242

# Logcat lines from the app's own process that mean a crash or a build that R8 broke.
PID_PATTERNS='FATAL EXCEPTION|JNI DETECTED ERROR|UnsatisfiedLinkError|ClassNotFoundException|NoClassDefFoundError|NoSuchMethodError|NoSuchFieldError|ExceptionInInitializerError|AbstractMethodError|VerifyError|Fatal signal|Abort message'
# Lines any process prints about the app (the system server reports ANRs, debuggerd native crashes).
PKG_RE='org\.mobileofficefree\.app'
GLOBAL_PATTERNS="Process: $PKG_RE, PID|ANR in $PKG_RE|>>> $PKG_RE <<<"

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
LOGCAT="$OUT/logcat.txt"
METRICS="$OUT/metrics.tsv"
RESULTS=()
RESULTS_TR=()
RESULTS_SAVE=()
RESULTS_RECOVERY=()
FAILED=0
SLOW=0
SLOW_TR=0
SLOW_SAVE=0
SLOW_RECOVERY=0
# The result of the last record() call.
LAST_RESULT=""
# 1: en-US as the emulator comes; 2: tr-TR, font scale 2.0, dark (second_pass); 3: saving (save_pass);
# 4: recovery (recovery_pass).
PASS=1
# How the file name is looked for in the title bar (ui_query mode).
TITLE_MODE=exact
RUN_STARTED=$(date +%s)

log() { printf '[smoke %s] %s\n' "$(date +%H:%M:%S)" "$*"; }

now_ms() { date +%s%3N; }
seconds() { awk -v ms="$1" 'BEGIN { printf "%.1f", ms / 1000 }'; }
megabytes() { awk -v kb="$1" 'BEGIN { printf "%.0f", kb / 1024 }'; }

# stdin from /dev/null: "adb shell" would otherwise swallow the rest of a while-read loop's input.
sh_adb() { timeout 90 adb shell "$@" < /dev/null; }

# --------------------------------------------------------------------------------------------
# UI tree helpers (uiautomator dump of the active window, read with Python on the host)

ui_dump() {  # ui_dump FILE: the active window's UI tree, or an empty file if uiautomator failed
    local file=$1
    : > "$file"
    for _ in 1 2 3; do
        sh_adb rm -f /sdcard/mo-smoke-window.xml > /dev/null 2>&1 || true
        if sh_adb uiautomator dump /sdcard/mo-smoke-window.xml > /dev/null 2>&1 &&
            timeout 30 adb exec-out cat /sdcard/mo-smoke-window.xml > "$file" 2> /dev/null < /dev/null &&
            grep -q '<hierarchy' "$file"; then
            return 0
        fi
        : > "$file"
        sleep 1
    done
    return 1
}

# ui_query FILE MODE LABEL...: MODE "contains" (a node's text or content-desc contains a label),
# "exact" (equals a label, or one comma-separated part of a merged node does), "title" (like exact,
# Unicode-normalised, and a value cut with "…" matches when what is left is a prefix of a label
# at least 6 characters long, or the whole label if shorter), "center" (prints "x y" of the first
# node containing a label), "center-exact" (the same for the first node matching like "exact"),
# "enabled" / "center-enabled" (like "exact" / "center-exact", skipping nodes that are, or sit in,
# a disabled control: the slide bar keeps a greyed-out Save), "show" (prints every value containing a label), "field-center" / "field-text" (no labels: "x y"
# or the text of the first text field, class EditText, on screen), "picker-save" / "picker-name"
# (no labels: "x y" of the system file picker's enabled save button, its android:id/button1 or a
# button reading "Save" in any case, or the text of its file name field), "after-field" (no labels:
# the first non-empty text after the first text field, outside it: the viewer's search counter).
# Exit status 0 when found.
ui_query() {
    python3 - "$@" << 'PY'
import re
import sys
import unicodedata
import xml.etree.ElementTree as ET


def center(node):
    x1, y1, x2, y2 = map(int, re.findall(r"\d+", node.get("bounds", "[0,0][0,0]")))
    return f"{(x1 + x2) // 2} {(y1 + y2) // 2}"


def title_hit(value, labels):
    value = unicodedata.normalize("NFC", value).strip()
    for label in (unicodedata.normalize("NFC", l) for l in labels):
        if value == label:
            return True
        if "…" in value or value.endswith("..."):
            prefix = value.split("…")[0].removesuffix("...").rstrip()
            if len(prefix) >= min(6, len(label)) and label.startswith(prefix):
                return True
    return False


path, mode, labels = sys.argv[1], sys.argv[2], sys.argv[3:]
found = False
try:
    root = ET.parse(path).getroot()
except (ET.ParseError, OSError):
    sys.exit(1)
nodes = list(root.iter("node"))
parents = {child: parent for parent in root.iter() for child in parent}


def disabled(node):
    while node is not None:
        if node.get("enabled") == "false":
            return True
        node = parents.get(node)
    return False


if mode in ("picker-save", "picker-name"):
    mine = [n for n in nodes if "documentsui" in n.get("package", "")]
    if mode == "picker-name":
        for node in mine:
            if node.get("class", "").endswith("EditText"):
                print(node.get("text") or "")
                sys.exit(0)
        sys.exit(1)
    for pick in (lambda n: n.get("resource-id", "").endswith(":id/button1"),
                 lambda n: (n.get("text") or "").strip().casefold() == "save"):
        for node in mine:
            if pick(node) and not disabled(node):
                print(center(node))
                sys.exit(0)
    sys.exit(1)
if mode == "after-field":
    field = next((n for n in nodes if n.get("class", "").endswith("EditText")), None)
    if field is None:
        sys.exit(1)
    inside = set(field.iter())
    passed = False
    for node in nodes:
        if node is field:
            passed = True
        elif passed and node not in inside and (node.get("text") or "").strip():
            print((node.get("text") or "").strip())
            sys.exit(0)
    sys.exit(1)
if mode in ("field-center", "field-text"):
    for node in nodes:
        if node.get("class", "").endswith("EditText"):
            print(center(node) if mode == "field-center" else node.get("text") or "")
            sys.exit(0)
    sys.exit(1)
for node in nodes:
    for value in (node.get("text") or "", node.get("content-desc") or ""):
        if not value:
            continue
        if mode in ("enabled", "center-enabled") and disabled(node):
            continue
        if mode in ("exact", "center-exact", "enabled", "center-enabled"):
            hit = any(value == l or l in [p.strip() for p in value.split(",")] for l in labels)
        elif mode == "title":
            hit = any(title_hit(p, labels) for p in [value] + value.split(","))
        else:
            hit = any(l in value for l in labels)
        if hit:
            if mode == "show":
                print(value[:300])
                found = True
                continue
            if mode in ("center", "center-exact", "center-enabled"):
                print(center(node))
            sys.exit(0)
sys.exit(0 if found else 1)
PY
}

# --------------------------------------------------------------------------------------------
# Process and logcat checks

alive() { [ -n "$(sh_adb pidof "$PKG" 2> /dev/null | tr -d '\r')" ]; }

log_mark() { sleep 1; wc -l < "$LOGCAT" | tr -d ' '; }

# check_logs STEP MARK: scans the logcat lines written since MARK; prints offenders, returns 1.
check_logs() {
    local step=$1 mark=$2 slice="$OUT/$1.logcat.txt" pids bad
    sleep 1
    tail -n +"$((mark + 1))" "$LOGCAT" > "$slice" || true
    # The app's processes during this step: started ones (ActivityManager) and the live one.
    pids=$( {
        grep -oE "Start proc [0-9]+:$PKG_RE/" "$slice" | grep -oE '[0-9]+' || true
        sh_adb pidof "$PKG" 2> /dev/null | tr -d '\r' | tr ' ' '\n' || true
    } | sort -u | tr '\n' ' ')
    bad=$( {
        if [ -n "${pids// /}" ]; then
            awk -v pids="$pids" 'BEGIN { n = split(pids, p, " "); for (i = 1; i <= n; i++) want[p[i]] = 1 } ($3 in want)' "$slice" |
                grep -E "$PID_PATTERNS" || true
        fi
        grep -E "$GLOBAL_PATTERNS" "$slice" || true
    } | { if [ -n "$LOG_IGNORE" ]; then grep -vE "$LOG_IGNORE" || true; else cat; fi; })
    if [ -n "$bad" ]; then
        echo "::group::logcat problems in step $step (app pids: ${pids:-none})"
        echo "$bad"
        echo "--- logcat of the step around them ---"
        grep -E -A40 "$PID_PATTERNS|$GLOBAL_PATTERNS" "$slice" | head -200 || true
        echo "::endgroup::"
        return 1
    fi
    return 0
}

# --------------------------------------------------------------------------------------------
# Measurements. A step fills the M_* variables it measured; record() writes them to the summary
# table and metrics.tsv, checks the soft budgets and clears them for the next step.

M_FIELDS=(M_SIZE M_BYTES M_STATE M_TOTAL M_WAIT M_SEEN_LOW M_SEEN M_BUDGET M_PSS M_JAVA M_NATIVE M_GRAPHICS)
# Their columns in metrics.tsv, after "step" and "result".
M_COLUMNS=(size_class file_bytes launch_state am_total_ms am_wait_ms visible_after_ms visible_by_ms open_budget_s
    pss_kb java_heap_kb native_heap_kb graphics_kb)
metrics_clear() { local v; for v in "${M_FIELDS[@]}"; do printf -v "$v" '%s' ""; done; }
metrics_clear

# am_times FILE: M_STATE, M_TOTAL and M_WAIT from the output of `am start -W`.
am_times() {
    M_STATE=$(sed -n 's/^LaunchState: *\([A-Z_]*\).*/\1/p' "$1" | head -1)
    M_TOTAL=$(sed -n 's/^TotalTime: *\([0-9][0-9]*\).*/\1/p' "$1" | head -1)
    M_WAIT=$(sed -n 's/^WaitTime: *\([0-9][0-9]*\).*/\1/p' "$1" | head -1)
}

# measure_memory STEP: M_PSS (TOTAL PSS) and the Java heap, native heap and graphics lines of
# the app summary, all in kB, from `dumpsys meminfo`; kept as STEP.meminfo.txt.
measure_memory() {
    local file="$OUT/$1.meminfo.txt"
    sh_adb dumpsys meminfo "$PKG" > "$file" 2>&1 || true
    M_PSS=$(sed -n 's/.*TOTAL PSS: *\([0-9][0-9]*\).*/\1/p' "$file" | head -1)
    # Older layouts have no "TOTAL PSS:" line; the first column of the TOTAL row is the PSS.
    [ -n "$M_PSS" ] || M_PSS=$(awk '$1 == "TOTAL" && $2 ~ /^[0-9]+$/ { print $2; exit }' "$file")
    M_JAVA=$(sed -n 's/^ *Java Heap: *\([0-9][0-9]*\).*/\1/p' "$file" | head -1)
    M_NATIVE=$(sed -n 's/^ *Native Heap: *\([0-9][0-9]*\).*/\1/p' "$file" | head -1)
    M_GRAPHICS=$(sed -n 's/^ *Graphics: *\([0-9][0-9]*\).*/\1/p' "$file" | head -1)
}

count_slow() {
    case "$PASS" in
        1) SLOW=$((SLOW + 1)) ;;
        2) SLOW_TR=$((SLOW_TR + 1)) ;;
        3) SLOW_SAVE=$((SLOW_SAVE + 1)) ;;
        *) SLOW_RECOVERY=$((SLOW_RECOVERY + 1)) ;;
    esac
}

record() {  # record STEP RESULT DETAIL
    local step=$1 result=$2 detail=$3 start="" visible="" pss=""
    if [ -n "$M_TOTAL" ]; then start="$M_TOTAL ms"; fi
    if [ -n "$M_SEEN" ] && [ -n "$M_SEEN_LOW" ]; then
        visible="$(seconds "$M_SEEN_LOW")–$(seconds "$M_SEEN") s"
    elif [ -n "$M_SEEN" ]; then
        visible="≤ $(seconds "$M_SEEN") s"
    fi
    if [ -n "$M_PSS" ]; then pss="$(megabytes "$M_PSS") MB"; fi
    if [ "$result" = PASS ] && [ -n "$M_SEEN" ] && [ -n "$M_BUDGET" ] && [ "$M_SEEN" -gt $((M_BUDGET * 1000)) ]; then
        count_slow
        visible="$visible ⚠"
        echo "::warning title=Slow open on the emulator: $step::visible after $(seconds "$M_SEEN") s, over the soft budget of $M_BUDGET s"
    fi
    if [ "$result" = PASS ] && [ -n "$M_PSS" ] && [ "$M_PSS" -gt $((PSS_BUDGET_MB * 1024)) ]; then
        count_slow
        pss="$pss ⚠"
        echo "::warning title=Large memory use on the emulator: $step::TOTAL PSS $(megabytes "$M_PSS") MB, over the soft budget of $PSS_BUDGET_MB MB"
    fi
    case "$PASS" in
        1) RESULTS+=("| $step | $result | $start | $visible | $pss | ${detail//|//} |") ;;
        2) RESULTS_TR+=("| $step | $result | $start | $visible | $pss | ${detail//|//} |") ;;
        3) RESULTS_SAVE+=("| $step | $result | $start | $visible | $pss | ${detail//|//} |") ;;
        *) RESULTS_RECOVERY+=("| $step | $result | $start | $visible | $pss | ${detail//|//} |") ;;
    esac
    LAST_RESULT=$result
    if [ -n "$M_TOTAL$M_SEEN$M_PSS" ]; then
        local v row=("$step" "$result")
        for v in "${M_FIELDS[@]}"; do row+=("${!v}"); done
        (IFS=$'\t'; printf '%s\n' "${row[*]}") >> "$METRICS"
    fi
    metrics_clear
    if [ "$result" = "FAIL" ]; then
        FAILED=$((FAILED + 1))
        echo "::error title=Emulator smoke test: $step::$detail"
    fi
    log "$step: $result - $detail${start:+ [am start $start]}${visible:+ [visible $visible]}${pss:+ [PSS $pss]}"
}

screenshot() { timeout 30 adb exec-out screencap -p > "$OUT/$1.png" 2> /dev/null < /dev/null || true; }

# After a crash the next start shows the "app closed unexpectedly" dialog once; start once to
# clear it so it does not hide the next document.
clear_crash_report() {
    sh_adb am force-stop "$PKG" || true
    sh_adb am start -W -n "$ACTIVITY" > /dev/null 2>&1 || true
    sleep 3
    sh_adb am force-stop "$PKG" || true
}

# --------------------------------------------------------------------------------------------
# Steps

# uri_encode NAME: NAME as one percent-encoded path segment (UTF-8), the way FileProvider
# writes it; ASCII letters, digits and "-_.~" stay as they are.
uri_encode() { python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$1"; }

# open_document NAME MIME ROUTE EXPECT SIZE [STEP]: cold start with a VIEW intent for one sample;
# SIZE "big" gives it the longer timeout and the larger time budget. STEP names the output files
# (default: from NAME).
open_document() {
    local name=$1 mime=$2 route=$3 expect=$4 size=${5:-small}
    local base="${name%.*}"
    local step="${6:-open-${base#mo-smoke-}}" uri
    uri="$URI_BASE/$(uri_encode "$name")"
    local xml="$OUT/$step.window.xml" mark started output deadline outcome="" detail="" notes_dismissed=0
    local open_timeout=$OPEN_TIMEOUT budget=$OPEN_BUDGET t0 dump_start last_miss="" seen=""
    if [ "$size" = big ]; then
        open_timeout=$BIG_OPEN_TIMEOUT
        budget=$BIG_OPEN_BUDGET
    fi
    sh_adb am force-stop "$PKG" || true
    mark=$(log_mark)
    started=$(date +%s)
    t0=$(now_ms)
    if [ "$route" = filter ]; then
        # Implicit, limited to the app: the manifest's intent filter has to accept the type.
        output=$(sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -p "$PKG" 2>&1 || true)
    else
        output=$(sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -n "$ACTIVITY" 2>&1 || true)
    fi
    echo "$output" > "$OUT/$step.am.txt"
    am_times "$OUT/$step.am.txt"
    M_SIZE=$size
    M_BYTES=$(stat -c %s "$SAMPLES/$name" 2> /dev/null || true)
    M_BUDGET=$budget
    if grep -qE '^Error|Exception|unable to resolve' <<< "$output"; then
        outcome=FAIL
        detail="the intent did not start the app: $(grep -m1 -E 'Error|Exception|unable' <<< "$output")"
    fi
    deadline=$((started + open_timeout))
    while [ -z "$outcome" ]; do
        if ! alive; then
            outcome=FAIL
            detail="the app process died while opening"
            break
        fi
        dump_start=$(now_ms)
        if ui_dump "$xml"; then
            if ui_query "$xml" contains "${FAILURE_TEXTS[@]}"; then
                outcome=FAIL
                detail="the app reported a problem on screen: $(ui_query "$xml" show "${FAILURE_TEXTS[@]}" | head -3 | tr '\n' ' ')"
                break
            fi
            if ui_query "$xml" exact "${BACK_LABELS[@]}" && ui_query "$xml" "$TITLE_MODE" "$base" "$name"; then
                outcome=PASS
                [ -n "$seen" ] || seen=$(($(now_ms) - t0))
                detail="document screen \"$base\" shown after $(($(date +%s) - started)) s"
                if [ "$PASS" = 2 ] && ! ui_query "$xml" exact "Geri"; then
                    outcome=FAIL
                    detail="$detail, but not in Turkish (no \"Geri\"): the $TR_LOCALE locale did not reach the app"
                fi
                break
            fi
            if [ "$expect" = viewer-or-notice ] && ui_query "$xml" contains "${NOTICE_TEXTS[@]}"; then
                outcome=PASS
                detail="the app announced the format as upcoming (no viewer yet)"
                break
            fi
            # Import notes are shown over the document after it opened: read them away once.
            if [ "$notes_dismissed" = 0 ] && ui_query "$xml" exact "${IMPORT_NOTES[@]}"; then
                # The document is open under them: this is when it became visible.
                seen=$(($(now_ms) - t0))
                log "$step: import notes shown, dismissing them"
                sh_adb input keyevent KEYCODE_BACK || true
                notes_dismissed=1
            elif [ -z "$seen" ]; then
                last_miss=$((dump_start - t0))
            fi
        fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            outcome=FAIL
            detail="no document screen for \"$base\" within $open_timeout s (on screen: $(ui_labels "$xml"); see $step.png / $step.window.xml)"
            break
        fi
        sleep "$POLL"
    done
    if [ "$outcome" = PASS ] && [ -n "$seen" ]; then
        M_SEEN=$seen
        # Before the first dump nothing was seen: the activity's first frame is the lower bound.
        M_SEEN_LOW=${last_miss:-$M_TOTAL}
    fi
    if [ "$outcome" = PASS ]; then
        # Pages render and engines keep working after the first frame: give a late crash time.
        sleep "$SETTLE"
        if ! alive; then
            outcome=FAIL
            detail="the app process died within $SETTLE s after the document appeared"
        else
            measure_memory "$step"
        fi
    fi
    screenshot "$step"
    if ! check_logs "$step" "$mark"; then
        outcome=FAIL
        detail="${detail}; crash signals in logcat (see the log above and $step.logcat.txt)"
    fi
    record "$step" "$outcome" "$detail"
    if [ "$outcome" = PASS ] && [ "$name" = mo-smoke-png.png ] && [ "$OCR" = 1 ] && [ "$PASS" = 1 ]; then
        recognise_text
    fi
    if [ "$outcome" = PASS ] && [ "$name" = mo-smoke-pdf-base14.pdf ] && [ "$PASS" = 1 ]; then
        check_pdf_letters "$step"
    fi
    if [ "$outcome" = FAIL ]; then clear_crash_report; fi
}

# colour_pixels PNG: "RED BLUE", the number of pure red and pure blue pixels in a screenshot
# (8-bit RGB or RGBA, not interlaced, as screencap writes it); status 1 if it cannot be read.
colour_pixels() {
    python3 - "$1" << 'PY'
import struct, sys, zlib
try:
    data = open(sys.argv[1], "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat = 8, b""
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and ctype in (2, 6) and not interlace
            bpp = 3 if ctype == 2 else 4
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
except Exception:
    sys.exit(1)
stride = width * bpp
prev = bytearray(stride)
red = blue = i = 0
for _ in range(height):
    f, line = raw[i], bytearray(raw[i + 1:i + 1 + stride])
    i += 1 + stride
    if f:
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            else:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
    for x in range(0, stride, bpp):
        r, g, bl = line[x], line[x + 1], line[x + 2]
        if r > 190 and g < 70 and bl < 70:
            red += 1
        elif bl > 190 and r < 70 and g < 70:
            blue += 1
    prev = line
print(red, blue)
PY
}

# Letters of fonts a PDF names without embedding (mo-smoke-pdf-base14.pdf, see
# make_smoke_samples.py): Android's renderer has no Ğ ğ İ Ş ş in its built-in Helvetica, so the
# app shows such a file from a copy with a stand-in font (rust/mo-pdf/src/standin.rs). The letters
# it would lose are pure red on the page, a control line it always draws pure blue: in the open
# step's screenshot the red pixels must be at least a third of the blue ones. Too little blue (the
# page not drawn, or not where the screenshot sees it) is a SKIP.
check_pdf_letters() {
    local step=pdf-base14-letters counts red blue
    if ! counts=$(colour_pixels "$OUT/$1.png"); then
        record "$step" SKIP "the screenshot $1.png could not be read"
        return
    fi
    read -r red blue <<< "$counts"
    if [ "$blue" -lt 500 ]; then
        record "$step" SKIP "the control letters are not in $1.png ($blue blue pixels): the page was not drawn"
    elif [ $((red * 3)) -lt "$blue" ]; then
        record "$step" FAIL "Ğ ğ İ Ş ş of a non-embedded Helvetica are missing on screen ($red red pixels for $blue blue control ones; see $1.png)"
    else
        record "$step" PASS "Ğ ğ İ Ş ş of a non-embedded Helvetica drawn ($red red pixels, $blue blue control ones)"
    fi
}

# Text recognition on the open picture: loads Tesseract's JNI library and its Java classes.
recognise_text() {
    local step=ocr-png xml="$OUT/ocr-png.window.xml" mark at deadline outcome="" detail=""
    ui_dump "$xml" || true
    if ! at=$(ui_query "$xml" center "${OCR_LABELS[@]}"); then
        record "$step" SKIP "no \"Recognise text\" button found on the picture screen"
        return
    fi
    mark=$(log_mark)
    # shellcheck disable=SC2086  # "x y"
    sh_adb input tap $at
    deadline=$(($(date +%s) + 90))
    while [ -z "$outcome" ]; do
        sleep 3
        if ! alive; then
            outcome=FAIL
            detail="the app process died during text recognition"
        elif ui_dump "$xml"; then
            if ui_query "$xml" contains "${FAILURE_TEXTS[@]}"; then
                outcome=FAIL
                detail="text recognition reported an error on screen"
            elif ui_query "$xml" contains "${OCR_DONE[@]}"; then
                outcome=PASS
                detail="text recognition finished"
            elif ! ui_query "$xml" contains "${OCR_WORKING[@]}" && [ "$(date +%s)" -ge "$deadline" ]; then
                outcome=FAIL
                detail="text recognition showed no result within 90 s"
            fi
        fi
        if [ -z "$outcome" ] && [ "$(date +%s)" -ge "$((deadline + 60))" ]; then
            outcome=FAIL
            detail="text recognition still running after 150 s"
        fi
    done
    screenshot "$step"
    if ! check_logs "$step" "$mark"; then
        outcome=FAIL
        detail="${detail}; crash signals in logcat"
    fi
    record "$step" "$outcome" "$detail"
}

# launch_home STEP: cold start of the home screen (the app's launcher entry).
launch_home() {
    local step=$1 xml="$OUT/$1.window.xml" mark started poll_start last_miss="" outcome=FAIL
    local detail="the main screen did not come up within 30 s"
    sh_adb am force-stop "$PKG" || true
    mark=$(log_mark)
    started=$(now_ms)
    sh_adb am start -W -n "$ACTIVITY" > "$OUT/$step.am.txt" 2>&1 || true
    am_times "$OUT/$step.am.txt"
    M_BUDGET=$OPEN_BUDGET
    while [ $(($(now_ms) - started)) -lt 30000 ]; do
        sleep "$POLL"
        alive || { detail="the app process died at start"; break; }
        poll_start=$(now_ms)
        if sh_adb dumpsys window 2> /dev/null | grep -E 'mCurrentFocus|mFocusedApp' | grep -q "$PKG"; then
            outcome=PASS
            M_SEEN=$(($(now_ms) - started))
            M_SEEN_LOW=${last_miss:-$M_TOTAL}
            detail="main screen focused after $(seconds "$M_SEEN") s"
            break
        fi
        last_miss=$((poll_start - started))
    done
    if [ "$outcome" = PASS ]; then
        sleep "$SETTLE"
        if alive; then
            measure_memory "$step"
        else
            outcome=FAIL
            detail="the app process died within $SETTLE s after start"
        fi
        if ui_dump "$xml" && ui_query "$xml" contains "${FAILURE_TEXTS[@]}"; then
            outcome=FAIL
            detail="the main screen shows a problem message"
        fi
    fi
    screenshot "$step"
    check_logs "$step" "$mark" || { outcome=FAIL; detail="${detail}; crash signals in logcat"; }
    record "$step" "$outcome" "$detail"
}

# --------------------------------------------------------------------------------------------
# Second pass: tr-TR, font scale 2.0, dark theme

ORIG_FONT_SCALE=""
ORIG_NIGHT=""
ORIG_LOCALES=""
DEVICE_CHANGED=0
PASS2_STARTED=""
PASS2_ENDED=""

# The app's own locales ("" when it follows the system), from `cmd locale get-app-locales`,
# which prints "Locales for PKG for user 0 are [tr-TR]".
app_locales() {
    sh_adb cmd locale get-app-locales "$PKG" 2> /dev/null | tr -d '\r' | sed -n 's/.*are \[\(.*\)\].*/\1/p' | head -1 || true
}

night_mode() { sh_adb cmd uimode night 2> /dev/null | tr -d '\r' | sed -n 's/^Night mode: *//p' | head -1 || true; }

font_scale() { sh_adb settings get system font_scale 2> /dev/null | tr -d '\r' || true; }

# Puts the font scale, night mode and the app's locales back as they were before the second pass.
restore_device() {
    [ "$DEVICE_CHANGED" = 1 ] || return 0
    DEVICE_CHANGED=0
    log "restoring font scale ${ORIG_FONT_SCALE:-1.0}, night mode ${ORIG_NIGHT:-no}, app locales [${ORIG_LOCALES}]"
    case "$ORIG_FONT_SCALE" in
        "" | null) sh_adb settings put system font_scale 1.0 || true ;;
        *) sh_adb settings put system font_scale "$ORIG_FONT_SCALE" || true ;;
    esac
    case "$ORIG_NIGHT" in
        yes | no | auto) sh_adb cmd uimode night "$ORIG_NIGHT" > /dev/null || true ;;
        *) sh_adb cmd uimode night no > /dev/null || true ;;
    esac
    if [ -n "$ORIG_LOCALES" ]; then
        sh_adb cmd locale set-app-locales "$PKG" --locales "$ORIG_LOCALES" || true
    else
        # Without --locales the app follows the system language again.
        sh_adb cmd locale set-app-locales "$PKG" || true
    fi
}

# The language check of the home screen at the start of the pass (the documents check "Geri").
check_home_language() {
    local xml="$OUT/tr-launch.window.xml"
    if [ ! -s "$xml" ]; then
        record tr-language SKIP "no UI tree of the home screen to check the language on"
    elif ui_query "$xml" contains "${HOME_TR[@]}"; then
        record tr-language PASS "home screen in Turkish: $(ui_query "$xml" show "${HOME_TR[@]}" | head -2 | tr '\n' ' ')"
    elif ui_query "$xml" contains "${HOME_EN[@]}"; then
        record tr-language FAIL "home screen still in English with app locales [$(app_locales)]: $(ui_query "$xml" show "${HOME_EN[@]}" | head -2 | tr '\n' ' ')"
    else
        record tr-language SKIP "neither the Turkish nor the English home screen texts found (see tr-launch.png)"
    fi
}

second_pass() {
    local list="$SAMPLES/samples-tr.tsv" elapsed name mime route expect size step got_scale got_night got_locales problems=""
    PASS=2
    TITLE_MODE=title
    METRICS="$OUT/metrics-tr.tsv"
    (IFS=$'\t'; columns=(step result "${M_COLUMNS[@]}"); printf '%s\n' "${columns[*]}") > "$METRICS"
    PASS2_STARTED=$(date +%s)
    elapsed=$(((PASS2_STARTED - RUN_STARTED) / 60))
    if [ ! -s "$list" ]; then
        record tr-setup SKIP "no samples-tr.tsv in $SAMPLES"
        return
    fi
    if [ "$elapsed" -ge $((TIME_LIMIT - 6)) ]; then
        echo "::warning title=Emulator smoke test: second pass skipped::the first pass took $elapsed min (limit $TIME_LIMIT min)"
        record tr-setup SKIP "skipped: the first pass took $elapsed min, too close to the $TIME_LIMIT min limit"
        return
    fi
    log "second pass: $TR_LOCALE, font scale $TR_FONT_SCALE, dark theme"
    # Names with Turkish letters and spaces: adb's sync protocol takes them as they are.
    while IFS=$'\t' read -r name _ <&3; do
        timeout 60 adb push "$SAMPLES/$name" "$DEVICE_DIR/$name" > /dev/null < /dev/null
    done 3< "$list"
    sh_adb ls -la "$DEVICE_DIR"

    ORIG_FONT_SCALE=$(font_scale)
    ORIG_NIGHT=$(night_mode)
    ORIG_LOCALES=$(app_locales)
    DEVICE_CHANGED=1
    sh_adb am force-stop "$PKG" || true
    sh_adb cmd locale set-app-locales "$PKG" --locales "$TR_LOCALE" || true
    sh_adb settings put system font_scale "$TR_FONT_SCALE" || true
    sh_adb cmd uimode night yes > /dev/null || true
    # The system redraws in the dark theme and the new configuration reaches every process.
    sleep 5
    got_scale=$(font_scale)
    got_night=$(night_mode)
    got_locales=$(app_locales)
    [[ "$got_locales" == *"${TR_LOCALE%%-*}"* ]] || problems+=" app locales are [$got_locales], not [$TR_LOCALE];"
    awk -v a="$got_scale" -v b="$TR_FONT_SCALE" 'BEGIN { exit !(a + 0 == b + 0) }' || problems+=" font scale is \"$got_scale\";"
    [ "$got_night" = yes ] || problems+=" night mode is \"$got_night\";"
    if [ -n "$problems" ]; then
        record tr-setup FAIL "the device did not take the settings:${problems%;}"
    else
        record tr-setup PASS "app locales [$got_locales], font scale $got_scale, night mode $got_night"
    fi

    launch_home tr-launch
    check_home_language
    while IFS=$'\t' read -r name mime route expect size step <&3; do
        if [ $((($(date +%s) - RUN_STARTED) / 60)) -ge "$TIME_LIMIT" ]; then
            record "$step" SKIP "skipped: the run reached its $TIME_LIMIT min limit"
            continue
        fi
        open_document "$name" "$mime" "$route" "$expect" "${size:-small}" "$step"
    done 3< "$list"
    sh_adb am force-stop "$PKG" || true
    restore_device
    PASS2_ENDED=$(date +%s)
}

# --------------------------------------------------------------------------------------------
# Third pass: edit, save over the file, check it with adb ("Kaydetme")

SAVE_STARTED=""
SAVE_ENDED=""
# 1 once save-xlsx showed that the cell edit brings up the top bar's Save button.
SHEET_EDIT_SEEN=0
# A save case's outcome as its driver leaves it: S_RESULT is PASS, FAIL, SKIP or WRITTEN (the file
# changed; save_case then checks S_COPY, the pulled file), S_DETAIL says what happened.
S_RESULT=""
S_DETAIL=""
S_COPY=""

save_note() { S_DETAIL="${S_DETAIL:+$S_DETAIL; }$*"; }
save_outcome() { S_RESULT=$1; save_note "$2"; }

hash_of() { sha256sum "$1" 2> /dev/null | cut -d' ' -f1; }

# pull_sample NAME DEST: copies the device's NAME to DEST; status 1 if adb could not read it.
pull_sample() {
    rm -f "$2"
    timeout 60 adb pull "$DEVICE_DIR/$1" "$2" > /dev/null 2>&1 < /dev/null && [ -f "$2" ]
}

# wait_written NAME HASH SECONDS: pulls NAME to S_COPY until its hash is not HASH (status 0) or
# SECONDS have passed (status 1).
wait_written() {
    local deadline=$(($(date +%s) + $3))
    while :; do
        if pull_sample "$1" "$S_COPY" && [ "$(hash_of "$S_COPY")" != "$2" ]; then return 0; fi
        [ "$(date +%s)" -lt "$deadline" ] || return 1
        sleep 2
    done
}

tap_at() { sh_adb input tap "${1% *}" "${1#* }"; }

# tap_label STEP MODE LABEL...: taps the first node matching (ui_query center MODE); status 1 if none.
tap_label() {
    local xml="$OUT/$1.window.xml" mode=$2 at
    shift 2
    ui_dump "$xml" || return 1
    at=$(ui_query "$xml" "$mode" "$@") || return 1
    tap_at "$at"
}

# wait_label STEP SECONDS MODE LABEL...: polls the screen until a node matches; status 1 if none did.
wait_label() {
    local xml="$OUT/$1.window.xml" deadline=$(($(date +%s) + $2)) mode=$3
    shift 3
    while :; do
        if ui_dump "$xml" && ui_query "$xml" "$mode" "$@"; then return 0; fi
        [ "$(date +%s)" -lt "$deadline" ] || return 1
        sleep 1
    done
}

field_text() { ui_query "$OUT/$1.window.xml" field-text 2> /dev/null || true; }

# Ctrl+S as a keyboard sends it; MainActivity turns it into Shortcut.SAVE.
press_ctrl_s() { sh_adb input keycombination KEYCODE_CTRL_LEFT KEYCODE_S > /dev/null 2>&1 || true; }

# The system's file picker ("Save as") in front: prints its package.
picker_package() { sh_adb dumpsys window 2> /dev/null | grep -m1 'mCurrentFocus' | grep -oiE '[a-z.]*documentsui' | head -n 1 || true; }

# close_picker: Back out of the picker (the app gets "cancelled"; the first Back may only close
# the keyboard over its file name field); stopped if it stays.
close_picker() {
    local pkg
    for _ in 1 2; do
        sh_adb input keyevent KEYCODE_BACK || true
        sleep 2
        pkg=$(picker_package)
        [ -n "$pkg" ] || return 0
    done
    sh_adb am force-stop "$pkg" || true
}

# verify_saved CASE ORIGINAL SAVED: what the pulled file holds; status 1 when it is not what
# saving in place must write.
verify_saved() {
    python3 - "$@" "$TYPED_WORD" "$TYPED_NUMBER" << 'PY'
import collections
import re
import shutil
import subprocess
import sys
import zipfile
import zlib

case, original, saved, word, number = sys.argv[1:6]
old = open(original, "rb").read()
new = open(saved, "rb").read()
good, bad = [], []


def check(ok, yes, no):
    (good if ok else bad).append(yes if ok else no)


def package(required):
    try:
        z = zipfile.ZipFile(saved)
    except zipfile.BadZipFile as e:
        bad.append(f"not a ZIP package any more ({e})")
        return None
    broken = z.testzip()
    check(broken is None, f"valid ZIP ({len(z.namelist())} parts)", f"ZIP entry {broken} fails its CRC check")
    missing = [n for n in required if n not in z.namelist()]
    check(not missing, "its main parts are there", "parts missing: " + ", ".join(missing))
    return z


def part(z, name):
    try:
        return z.read(name).decode("utf-8")
    except (KeyError, UnicodeDecodeError):
        return ""


if case == "docx":
    z = package(["[Content_Types].xml", "word/document.xml"])
    if z:
        text = "".join(re.findall(r"<w:t(?:\s[^>]*)?>([^<]*)</w:t>", part(z, "word/document.xml")))
        check(word in text, f"holds the typed \"{word}\"", f"the typed \"{word}\" is not in word/document.xml")
        check("Duman testi" in text and "İstanbul" in text, "the old text is kept", "the old text (\"Duman testi\", \"İstanbul\") is missing")
elif case == "xlsx":
    z = package(["[Content_Types].xml", "xl/workbook.xml"])
    if z:
        xml = "".join(part(z, n) for n in z.namelist() if n.startswith("xl/worksheets/") or n == "xl/sharedStrings.xml")
        check(re.search(rf">{number}<", xml), f"holds the typed {number}", f"the typed {number} is not in the sheet")
        check("İstanbul" in xml and "Ağrı" in xml, "the other cells are kept", "other cells (\"İstanbul\", \"Ağrı\") are missing")
elif case == "pptx":
    z = package(["[Content_Types].xml", "ppt/presentation.xml"])
    if z:
        def slides(pkg):
            names = [n for n in pkg.namelist() if re.fullmatch(r"ppt/slides/slide\d+\.xml", n)]
            return {n: re.findall(r"<a:t(?:\s[^>]*)?>([^<]*)</a:t>", part(pkg, n)) for n in names}

        before, after = slides(zipfile.ZipFile(original)), slides(z)
        listed = len(re.findall(r"<p:sldId\s", part(z, "ppt/presentation.xml")))
        check(len(after) == len(before) == listed, f"{len(after)} slides as before", f"{len(after)} slide parts and {listed} listed, not {len(before)}")
        typed_on = [n for n, texts in after.items() if word in texts]
        check(len(typed_on) == 1, f"holds the typed \"{word}\" on {''.join(typed_on)}", f"the typed \"{word}\" is on {len(typed_on)} slides, not 1")
        old_texts = collections.Counter(t for texts in before.values() for t in texts)
        new_texts = collections.Counter(t for texts in after.values() for t in texts)
        new_texts[word] = max(new_texts[word] - 1, 0)
        lost = old_texts - new_texts
        check(not lost, f"every old text kept ({sum(old_texts.values())})", "old texts missing: " + ", ".join(sorted(lost)))
elif case in ("txt-utf8", "txt-1254"):
    encoding = "utf-8" if case == "txt-utf8" else "cp1254"
    check(new.count(word.encode()) == 1, f"holds the typed \"{word}\" once", f"\"{word}\" is in it {new.count(word.encode())} times")
    check(new.startswith(b"\xef\xbb\xbf") == old.startswith(b"\xef\xbb\xbf"), "no byte order mark added", "a UTF-8 byte order mark was added or removed")
    if b"\r\n" in old:
        check(new.count(b"\n") == new.count(b"\r\n"), "CRLF line ends kept", "line ends changed (bare LF in a CRLF file)")
    else:
        check(b"\r" not in new, "LF line ends kept", "CR added to the line ends")
    letters = "ığüşöç İĞÜŞÖÇ"
    check(letters.encode(encoding) in new, f"Turkish letters still in {encoding}", f"the Turkish letters are no longer {encoding} bytes")
    if encoding != "utf-8":
        check(letters.encode("utf-8") not in new, "not re-encoded to UTF-8", "the Turkish letters were re-encoded to UTF-8")
    rest = new.replace(word.encode(), b"", 1)
    first = next((i for i, (a, b) in enumerate(zip(rest, old)) if a != b), min(len(rest), len(old)))
    check(rest == old, "every other byte as before", f"other bytes changed too (from byte {first}; {len(old)} → {len(rest)} B without the typed word)")
elif case == "pdf":
    # A new file (the opened one is not written): a complete PDF holding the filled-in value and
    # every field and page of the form. PdfCraft writes objects into compressed object streams,
    # so the streams are inflated before looking.
    def readable(data):
        parts = [data]
        for m in re.finditer(rb"stream\r?\n(.*?)endstream", data, re.S):
            try:
                parts.append(zlib.decompressobj().decompress(m.group(1)))
            except zlib.error:
                pass  # not Flate (or not compressed): already in the raw bytes
        return b"\n".join(parts)

    def pages(text):
        return len(re.findall(rb"/Type\s*/Page(?![A-Za-z])", text))

    def text_of(path):
        try:
            run = subprocess.run(["pdftotext", "-q", path, "-"], capture_output=True, timeout=60)
        except (OSError, subprocess.TimeoutExpired):
            return None
        return run.stdout.decode("utf-8", "replace") if run.returncode == 0 else None

    check(new.startswith(b"%PDF-"), f"a PDF ({new[:8].decode('latin-1').strip()}, {len(new)} B)", f"no %PDF- header (starts {new[:8]!r})")
    check(b"%%EOF" in new[-1024:], "ends with %%EOF (written whole)", "no %%EOF at the end: cut short")
    before, after = readable(old), readable(new)
    check(re.search(rb"/V\s*\(" + word.encode() + rb"\)", after), f"the field's value is \"{word}\"", f"no field value (/V ({word})) in the PDF")
    names = sorted(set(re.findall(rb"/T\s*\(([^()]*)\)", before)))
    lost = [n.decode("latin-1") for n in names if not re.search(rb"/T\s*\(" + re.escape(n) + rb"\)", after)]
    check(names and not lost, f"its {len(names)} fields kept", "fields missing: " + (", ".join(lost) or "none found in the original"))
    check(pages(after) == pages(before) > 0, f"{pages(after)} page(s) as before", f"{pages(after)} pages, not {pages(before)}")
    if shutil.which("pdftotext"):
        shown, was = text_of(saved), text_of(original)
        if shown is None:
            bad.append("pdftotext could not read it")
        else:
            check(word in shown, f"pdftotext shows \"{word}\"", f"pdftotext does not show \"{word}\"")
            kept = [line for line in (was or "").split("\n") if line.strip() and line.strip() not in shown]
            check(not kept, "and the old text", "pdftotext no longer shows: " + ", ".join(kept[:3]))
    else:
        good.append("pdftotext not installed: text not rendered")
else:
    bad.append(f"no check for case {case}")
print("; ".join(bad + good))
sys.exit(1 if bad else 0)
PY
}

# word_edit STEP: "Edit", then the word typed at the caret (the start of the document). Status 1
# (and a SKIP) when the editor does not come up.
word_edit() {
    local step=$1
    tap_label "$step" center-exact "${EDIT_LABELS[@]}" || { save_outcome SKIP "no \"Edit\" on the document screen"; return 1; }
    wait_label "$step" 15 exact "${KEYBOARD_LABELS[@]}" || { save_outcome SKIP "the editing tool bar did not come up after \"Edit\""; return 1; }
    sleep 1
    sh_adb input text "$TYPED_WORD"
    sleep 2
}

# save_word STEP NAME HASH: word_edit, Ctrl+S; if that did not write, the top bar's Save (shown after
# any change, also while editing), or, after Back, the leave dialog's Save.
save_word() {
    local step=$1 name=$2 old=$3 xml="$OUT/$1.window.xml" how="" picker
    word_edit "$step" || return 0
    press_ctrl_s
    if wait_written "$name" "$old" 15; then
        how="Ctrl+S"
    else
        picker=$(picker_package)
        if [ -n "$picker" ]; then
            close_picker
            save_outcome FAIL "Ctrl+S opened \"Save as\" ($picker) instead of saving over the file: in-place save is off for a clean .docx"
            return
        fi
        # Save is in the top bar after the change; Back (the first may only close the keyboard)
        # ends in the leave dialog, which has its own Save.
        for _ in 1 2 3; do
            if ui_dump "$xml" && ui_query "$xml" exact "${SAVE_LABELS[@]}"; then
                if ui_query "$xml" exact "${LEAVE_DISCARD_LABELS[@]}"; then
                    # The leave dialog: Save, Save as…, Exit without saving and Keep editing.
                    if ui_query "$xml" exact "${SAVE_AS_LABELS[@]}" && ui_query "$xml" exact "${LEAVE_STAY_LABELS[@]}"; then
                        save_note "the leave dialog (Save, Save as…, Exit without saving, Keep editing)"
                    else
                        save_note "the leave dialog lacks \"Save as…\" or \"Keep editing\""
                    fi
                fi
                break
            fi
            sh_adb input keyevent KEYCODE_BACK || true
            sleep 2
        done
        if ! tap_label "$step" center-exact "${SAVE_LABELS[@]}"; then
            save_outcome SKIP "Ctrl+S did not write and no \"Save\" button came up after leaving editing (did the typed text reach the document?)"
            return
        fi
        if wait_written "$name" "$old" 20; then
            how="the Save button (Ctrl+S did not write)"
        else
            picker=$(picker_package)
            [ -z "$picker" ] || close_picker
            save_outcome FAIL "the file did not change after Ctrl+S and the Save button${picker:+ (\"Save as\" opened instead)}"
            return
        fi
    fi
    save_outcome WRITTEN "typed \"$TYPED_WORD\", saved with $how"
}

# sheet_edit STEP: A1 (selected on opening) changed in the formula bar: its text deleted,
# TYPED_NUMBER typed, Enter. Status 1 (and a SKIP) without a formula bar field.
sheet_edit() {
    local step=$1 xml="$OUT/$1.window.xml" at before keys=(KEYCODE_MOVE_END)
    ui_dump "$xml" || true
    at=$(ui_query "$xml" field-center) || { save_outcome SKIP "no formula bar field on the sheet screen"; return 1; }
    before=$(field_text "$step")
    tap_at "$at"
    sleep 1
    for _ in $(seq 1 24); do keys+=(KEYCODE_DEL); done
    sh_adb input keyevent "${keys[@]}"
    sh_adb input text "$TYPED_NUMBER"
    sleep 1
    sh_adb input keyevent KEYCODE_ENTER
    sleep 2
    save_note "A1 \"$before\" → $TYPED_NUMBER in the formula bar, Enter"
}

# save_sheet STEP NAME HASH: the cell edit, then the top bar's Save; the "Saved" snackbar is noted.
save_sheet() {
    local step=$1 name=$2 old=$3 xml="$OUT/$1.window.xml" said="" deadline
    sheet_edit "$step" || return 0
    if ! wait_label "$step" 10 exact "${SAVE_LABELS[@]}"; then
        save_outcome SKIP "no \"Save\" in the top bar after the edit (field shows \"$(field_text "$step")\"): the edit was not committed"
        return
    fi
    SHEET_EDIT_SEEN=1
    tap_label "$step" center-exact "${SAVE_LABELS[@]}" || { save_outcome SKIP "the \"Save\" button went away before it could be tapped"; return; }
    deadline=$(($(date +%s) + 20))
    while :; do
        if [ -z "$said" ] && ui_dump "$xml"; then
            if ui_query "$xml" contains "${SAVE_PROBLEM_TEXTS[@]}"; then
                save_outcome FAIL "the app reported: $(ui_query "$xml" show "${SAVE_PROBLEM_TEXTS[@]}" | head -1)"
                return
            fi
            if ui_query "$xml" exact "${SAVED_TEXTS[@]}"; then said="\"Saved\" shown"; fi
        fi
        if pull_sample "$name" "$S_COPY" && [ "$(hash_of "$S_COPY")" != "$old" ]; then
            # Written: one more look for the snackbar, which stays a few seconds.
            if [ -z "$said" ] && ui_dump "$xml" && ui_query "$xml" exact "${SAVED_TEXTS[@]}"; then said="\"Saved\" shown"; fi
            save_outcome WRITTEN "saved with the top bar's Save, ${said:-no \"Saved\" snackbar seen}"
            return
        fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            save_outcome FAIL "the file did not change within 20 s of tapping Save (${said:-no message})"
            return
        fi
        sleep 1
    done
}

# block_sheet STEP NAME HASH: a workbook that must not be saved over: the same edit, then the top
# bar's Save must come up and open "Save as" (cancelled), Ctrl+S too, and the bytes stay.
block_sheet() {
    local step=$1 name=$2 old=$3 picker shown how="" ctrl_s="did not write"
    sheet_edit "$step" || return 0
    if ! wait_label "$step" 10 exact "${SAVE_LABELS[@]}"; then
        if [ "$SHEET_EDIT_SEEN" = 1 ]; then
            save_outcome FAIL "no \"Save\" in the top bar after the edit (field shows \"$(field_text "$step")\"): it must show and lead to \"Save as\""
        else
            save_outcome SKIP "no \"Save\" in the top bar after the edit, and save-xlsx did not show that this edit brings it up"
        fi
        return
    fi
    shown=$(field_text "$step")
    if tap_label "$step" center-exact "${SAVE_LABELS[@]}"; then
        sleep 4
        picker=$(picker_package)
        if [ -n "$picker" ]; then
            how="the top bar's Save opened \"Save as\" (cancelled)"
            close_picker
        fi
    fi
    press_ctrl_s
    sleep 4
    picker=$(picker_package)
    if [ -n "$picker" ]; then
        ctrl_s="opened \"Save as\" (cancelled)"
        close_picker
    fi
    sleep 2
    if ! pull_sample "$name" "$S_COPY"; then
        save_outcome SKIP "adb could not pull the file back"
    elif [ "$(hash_of "$S_COPY")" != "$old" ]; then
        save_outcome FAIL "the file was written over (its hash changed) although the engine did not read it whole"
    elif [ -z "$how" ]; then
        save_outcome FAIL "the top bar's \"Save\" neither wrote nor opened \"Save as\" (Ctrl+S $ctrl_s; the file is unchanged)"
    else
        save_outcome PASS "field shows \"$shown\", $how, Ctrl+S $ctrl_s, the bytes are unchanged"
    fi
}

# slide_edit STEP: "Edit", the slide bar's "Text box", the word typed into its dialog's field,
# "Apply": a new text box on the slide shown (the first after opening), made on the engine thread;
# an enabled "Save" comes up once the slides are edited (the slide bar greys its own out until
# then). Status 1 (and a SKIP) when a step is not there.
slide_edit() {
    local step=$1 xml="$OUT/$1.window.xml" at typed
    tap_label "$step" center-exact "${EDIT_LABELS[@]}" || { save_outcome SKIP "no \"Edit\" on the slides screen"; return 1; }
    wait_label "$step" 10 exact "${TEXT_BOX_LABELS[@]}" || { save_outcome SKIP "the slide editing bar (\"Text box\") did not come up after \"Edit\""; return 1; }
    tap_label "$step" center-exact "${TEXT_BOX_LABELS[@]}" || { save_outcome SKIP "\"Text box\" went away before it could be tapped"; return 1; }
    sleep 2
    ui_dump "$xml" || true
    at=$(ui_query "$xml" field-center) || { save_outcome SKIP "no text field in the \"Text box\" dialog"; return 1; }
    tap_at "$at"
    sleep 1
    sh_adb input text "$TYPED_WORD"
    sleep 1
    ui_dump "$xml" || true
    typed=$(field_text "$step")
    [[ "$typed" == *"$TYPED_WORD"* ]] || { save_outcome SKIP "the typed \"$TYPED_WORD\" did not reach the dialog's field (it shows \"$typed\")"; return 1; }
    tap_label "$step" center-exact "${APPLY_LABELS[@]}" || { save_outcome SKIP "no \"Apply\" in the \"Text box\" dialog"; return 1; }
    if ! wait_label "$step" 10 enabled "${SAVE_LABELS[@]}"; then
        save_outcome SKIP "no enabled \"Save\" after \"Apply\": the text box was not added"
        return 1
    fi
    save_note "\"Edit\", \"Text box\", typed \"$TYPED_WORD\", \"Apply\""
}

# save_slides STEP NAME HASH: slide_edit, then the top bar's Save must write the file.
save_slides() {
    local step=$1 name=$2 old=$3 picker
    slide_edit "$step" || return 0
    tap_label "$step" center-enabled "${SAVE_LABELS[@]}" || { save_outcome SKIP "the \"Save\" button went away before it could be tapped"; return; }
    if wait_written "$name" "$old" 20; then
        save_outcome WRITTEN "saved with the top bar's Save"
        return
    fi
    picker=$(picker_package)
    if [ -n "$picker" ]; then
        close_picker
        save_outcome FAIL "Save opened \"Save as\" ($picker) instead of saving over the file: in-place save is off for a clean .pptx"
    else
        save_outcome FAIL "the file did not change within 20 s of tapping Save"
    fi
}

# expect_save_as STEP NAME HASH: after an edit of a document the app must not save over (Word,
# slides): Ctrl+S and, if no "Save as" came up and nothing was written, the top bar's Save (Word
# shows it once editing is left; the first Back may only close the keyboard) must open the
# system's "Save as", which is cancelled; the file must keep its bytes. Only "Save as" coming up
# shows that the guard kept the file, rather than a key press or tap that went nowhere.
expect_save_as() {
    local step=$1 name=$2 old=$3 xml="$OUT/$1.window.xml" picker how=""
    press_ctrl_s
    sleep 4
    picker=$(picker_package)
    if [ -n "$picker" ]; then
        how="Ctrl+S opened \"Save as\" (cancelled)"
        close_picker
    elif pull_sample "$name" "$S_COPY" && [ "$(hash_of "$S_COPY")" = "$old" ]; then
        for _ in 1 2 3; do
            if ui_dump "$xml" && ui_query "$xml" enabled "${SAVE_LABELS[@]}"; then break; fi
            sh_adb input keyevent KEYCODE_BACK || true
            sleep 2
        done
        if tap_label "$step" center-enabled "${SAVE_LABELS[@]}"; then
            sleep 4
            picker=$(picker_package)
            if [ -n "$picker" ]; then
                how="Ctrl+S did not, but the Save button opened \"Save as\" (cancelled)"
                close_picker
            fi
        fi
    fi
    sleep 2
    if ! pull_sample "$name" "$S_COPY"; then
        save_outcome SKIP "adb could not pull the file back"
    elif [ "$(hash_of "$S_COPY")" != "$old" ]; then
        save_outcome FAIL "the file was written over (its hash changed) although the engine did not read it whole"
    elif [ -n "$how" ]; then
        save_outcome PASS "$how, the bytes are unchanged"
    else
        save_outcome SKIP "the bytes are unchanged, but neither Ctrl+S nor a Save button opened \"Save as\", so this proves nothing about the guard"
    fi
}

block_word() {
    word_edit "$1" || return 0
    save_note "typed \"$TYPED_WORD\" after \"Edit\""
    expect_save_as "$@"
}

block_slides() {
    slide_edit "$1" || return 0
    expect_save_as "$@"
}

# app_log TAG [TEXT]: the app's last logcat lines with TAG (those containing TEXT), on one line.
app_log() { sh_adb logcat -d -s "$1:*" 2> /dev/null | grep -F -- "${2:-}" | tail -n 6 | tr '\n' ' ' || true; }

# text_edit STEP: "Edit", a tap in the text (below it: the caret goes to the end), the word typed,
# "Save". Status 1 (and a SKIP) when a step is not there.
text_edit() {
    local step=$1 xml="$OUT/$1.window.xml" at typed
    tap_label "$step" center-exact "${EDIT_LABELS[@]}" || { save_outcome SKIP "no \"Edit\" on the text screen"; return 1; }
    sleep 2
    ui_dump "$xml" || true
    at=$(ui_query "$xml" field-center) || { save_outcome SKIP "no text field after \"Edit\""; return 1; }
    tap_at "$at"
    sleep 1
    sh_adb input text "$TYPED_WORD"
    sleep 1
    ui_dump "$xml" || true
    typed=$(field_text "$step")
    [[ "$typed" == *"$TYPED_WORD"* ]] || { save_outcome SKIP "the typed \"$TYPED_WORD\" did not reach the text field"; return 1; }
    tap_label "$step" center-exact "${SAVE_LABELS[@]}" || { save_outcome SKIP "no \"Save\" while editing"; return 1; }
    save_note "typed \"$TYPED_WORD\", tapped Save"
}

save_text() {
    local step=$1 name=$2 old=$3
    text_edit "$step" || return 0
    if wait_written "$name" "$old" 15; then
        save_outcome WRITTEN "written"
    else
        # The app's own explanation (a toast uiautomator cannot see) is also logged.
        local why
        why=$(sh_adb logcat -d -s 'BadoText:*' 2>/dev/null | grep -E 'save|Exception|Error' | tail -n 6 | tr '\n' ' ' || true)
        save_outcome FAIL "the file did not change within 15 s of tapping Save${why:+ (app log: $why)}"
    fi
}

block_text() {
    local step=$1 name=$2 old=$3 picker how=""
    text_edit "$step" || return 0
    sleep 4
    # "Save" says why (a toast, which uiautomator does not see) and opens "Save as" instead.
    picker=$(picker_package)
    if [ -n "$picker" ]; then
        how="\"Save\" opened \"Save as\" (cancelled)"
        close_picker
    fi
    sleep 1
    if ! pull_sample "$name" "$S_COPY"; then
        save_outcome SKIP "adb could not pull the file back"
    elif [ "$(hash_of "$S_COPY")" != "$old" ]; then
        save_outcome FAIL "the file was written over although its line ends would not come back the same"
    elif [ -z "$how" ]; then
        save_outcome FAIL "the bytes are unchanged, but \"Save\" did not open \"Save as\" (it must lead there when it cannot write over the file)"
    else
        save_outcome PASS "$how, the bytes are unchanged"
    fi
}

# PDF: the opened file is never written; an edited PDF goes into a new file the person picks
# (PagedScreen's "Save" opens the system's "Save as" with "<name>-duzenlendi.pdf" suggested).

# device_find GLOB: the files in shared storage named like GLOB, one device path per line.
device_find() { sh_adb "find /storage/emulated/0/ -type f -name '$1' 2> /dev/null" 2> /dev/null | tr -d '\r' || true; }

# folder_list: the names in the samples' folder, sorted (to see whether anything was added to it).
folder_list() { sh_adb ls -a "$DEVICE_DIR" 2> /dev/null | tr -d '\r' | sort || true; }

# added_files BEFORE AFTER [SKIP_GLOB]: the names in the AFTER listing that are not in BEFORE
# (except those matching SKIP_GLOB), on one line.
added_files() {
    local name out=""
    while IFS= read -r name; do
        [ -n "$name" ] || continue
        if [ -n "${3:-}" ]; then
            # shellcheck disable=SC2053  # a glob on purpose
            [[ "$name" == $3 ]] && continue
        fi
        grep -qxF -- "$name" <<< "$1" || out="$out $name"
    done <<< "$2"
    printf '%s' "${out# }"
}

# pdf_complete FILE: the file ends like a PDF written whole (%%EOF in its last KiB).
pdf_complete() { [ "$(tail -c 1024 "$1" 2> /dev/null | grep -ac '%%EOF' || true)" -gt 0 ]; }

screen_width() {
    local w
    w=$(sh_adb wm size 2> /dev/null | tr -d '\r' | tail -n 1 | sed -n 's/.*: *\([0-9][0-9]*\)x[0-9]*.*/\1/p')
    echo "${w:-1080}"
}

# pdf_tool STEP LABEL...: taps a tool of the PDF screen's bottom bar, which scrolls sideways: the
# bar is swiped (at the height of its first tools) until the tool is on screen. Status 1 if it
# never is.
# ui_labels XML: the texts and content descriptions on screen, for a SKIP's detail.
ui_labels() {
    python3 - "$1" <<'PY' 2>/dev/null || true
import sys, xml.etree.ElementTree as ET
try:
    seen = []
    for n in ET.parse(sys.argv[1]).iter("node"):
        for k in ("text", "content-desc"):
            v = (n.get(k) or "").strip()
            if v and v not in seen and len(v) < 40:
                seen.append(v)
    print(" | ".join(seen[:40]))
except Exception:
    pass
PY
}

pdf_tool() {
    local step=$1 xml="$OUT/$1.window.xml" at y="" width
    shift
    width=$(screen_width)
    for _ in 1 2 3 4 5; do
        ui_dump "$xml" || return 1
        if at=$(ui_query "$xml" center-exact "$@"); then
            tap_at "$at"
            return 0
        fi
        if [ -z "$y" ]; then
            at=$(ui_query "$xml" center-exact "${TOOLBAR_LABELS[@]}") || return 1
            y=${at#* }
        fi
        # A slow drag (a fast one is a fling the bar may ignore on a busy emulator), from the
        # middle of the bar's height (the label's own centre sits low in the tool).
        sh_adb input touchscreen swipe $((width * 9 / 10)) "$((y - 20))" $((width / 10)) "$((y - 20))" 900 || true
        sleep 2
    done
    return 1
}

# original_kept NAME HASH: status 0 when the device's NAME still has HASH; ORIGINAL_STATE says what
# was found (status 1 also when adb could not read it back).
ORIGINAL_STATE=""
original_kept() {
    if ! pull_sample "$1" "$OUT/original-check.tmp"; then
        ORIGINAL_STATE="adb could not pull the opened file back"
        return 1
    fi
    if [ "$(hash_of "$OUT/original-check.tmp")" != "$2" ]; then
        ORIGINAL_STATE="the opened file was written over (its hash changed)"
        return 1
    fi
    ORIGINAL_STATE="the opened file keeps its bytes"
}

# save_pdf STEP NAME HASH: "Fill in form", the word typed into the first field (the form's text
# field), "Apply", then the top bar's "Save", which must open the system's "Save as" with
# "<name>-duzenlendi.pdf" suggested; its save button is tapped in the folder it opens on
# (Downloads on a fresh device). The new file is looked for in the whole shared storage and pulled
# into S_COPY; the opened file must keep its bytes and nothing else may appear next to it.
save_pdf() {
    local step=$1 name=$2 old=$3 xml="$OUT/$1.window.xml" at typed picker="" suggested before after added path out="" deadline
    local stem="${name%.*}-duzenlendi"
    # A copy left by an earlier run would make the picker call the new one "… (1)".
    while IFS= read -r path; do
        [ -z "$path" ] || sh_adb "rm -f '$path'" || true
    done <<< "$(device_find "$stem*.pdf")"
    before=$(folder_list)
    pdf_tool "$step" "${FORM_FILL_LABELS[@]}" || { save_outcome SKIP "no \"Fill in form\" in the PDF tool bar (on screen: $(ui_labels "$OUT/$step.window.xml"))"; return 0; }
    wait_label "$step" 15 exact "${APPLY_LABELS[@]}" || { save_outcome SKIP "the form sheet (\"Apply\") did not come up after \"Fill in form\""; return 0; }
    at=$(ui_query "$xml" field-center) || { save_outcome SKIP "no text field in the form sheet"; return 0; }
    tap_at "$at"
    sleep 1
    sh_adb input text "$TYPED_WORD"
    sleep 1
    ui_dump "$xml" || true
    typed=$(field_text "$step")
    [[ "$typed" == *"$TYPED_WORD"* ]] || { save_outcome SKIP "the typed \"$TYPED_WORD\" did not reach the form's first field (it shows \"$typed\")"; return 0; }
    tap_label "$step" center-enabled "${APPLY_LABELS[@]}" || { save_outcome SKIP "no enabled \"Apply\" in the form sheet"; return 0; }
    wait_label "$step" 20 enabled "${SAVE_LABELS[@]}" || { save_outcome SKIP "no enabled \"Save\" in the top bar after \"Apply\": the form was not filled"; return 0; }
    save_note "\"Fill in form\", typed \"$TYPED_WORD\" into its first field, \"Apply\""
    tap_label "$step" center-enabled "${SAVE_LABELS[@]}" || { save_outcome SKIP "the \"Save\" button went away before it could be tapped"; return 0; }
    deadline=$(($(date +%s) + 10))
    while [ -z "$picker" ] && [ "$(date +%s)" -lt "$deadline" ]; do
        sleep 1
        picker=$(picker_package)
    done
    if [ -z "$picker" ]; then
        if original_kept "$name" "$old"; then
            save_outcome SKIP "\"Save\" did not bring up the system's \"Save as\" within 10 s; $ORIGINAL_STATE"
        else
            save_outcome FAIL "\"Save\" did not bring up \"Save as\"; $ORIGINAL_STATE"
        fi
        return 0
    fi
    # The picker lists its folder and fills in the name.
    sleep 3
    ui_dump "$xml" || true
    suggested=$(ui_query "$xml" picker-name 2> /dev/null || true)
    if ! at=$(ui_query "$xml" picker-save); then
        close_picker
        if original_kept "$name" "$old"; then
            save_outcome SKIP "\"Save as\" ($picker) came up for \"$suggested\", but no enabled save button was found in it: the new file was not checked; $ORIGINAL_STATE"
        else
            save_outcome FAIL "$ORIGINAL_STATE (\"Save as\" came up, its save button was not found)"
        fi
        return 0
    fi
    tap_at "$at"
    save_note "\"Save\" opened \"Save as\" ($picker) for \"$suggested\", its save button tapped"
    deadline=$(($(date +%s) + 30))
    while :; do
        out=$(device_find "$stem*.pdf" | head -n 1)
        if [ -n "$out" ] && timeout 60 adb pull "$out" "$S_COPY" > /dev/null 2>&1 < /dev/null && pdf_complete "$S_COPY"; then break; fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            out=""
            break
        fi
        sleep 2
    done
    picker=$(picker_package)
    [ -z "$picker" ] || close_picker
    after=$(folder_list)
    added=$(added_files "$before" "$after" "$stem*")
    if ! original_kept "$name" "$old"; then
        save_outcome FAIL "$ORIGINAL_STATE"
    elif [ -n "$added" ]; then
        save_outcome FAIL "files appeared next to the opened one: $added"
    elif [ -n "$out" ]; then
        save_outcome WRITTEN "written as $out, $ORIGINAL_STATE"
    elif [ -n "$picker" ]; then
        save_outcome SKIP "\"Save as\" was still open after its save button was tapped (closed now): no new file to check; $ORIGINAL_STATE"
    else
        save_outcome FAIL "no complete $stem*.pdf appeared in shared storage within 30 s of saving in \"Save as\"; $ORIGINAL_STATE"
    fi
}

# refused_open STEP NAME MIME ROUTE HASH: a file the app must not open (a PDF without a readable
# page). A cold start with it must show the open failure with that reason instead of a document
# screen, keep the file's bytes and add nothing to its folder.
refused_open() {
    local step=$1 name=$2 mime=$3 route=$4 old=$5 xml="$OUT/$1.window.xml" uri output before after added deadline
    local seen=0 opened=0 shown=""
    uri="$URI_BASE/$(uri_encode "$name")"
    before=$(folder_list)
    sh_adb am force-stop "$PKG" || true
    if [ "$route" = filter ]; then
        output=$(sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -p "$PKG" 2>&1 || true)
    else
        output=$(sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -n "$ACTIVITY" 2>&1 || true)
    fi
    echo "$output" > "$OUT/$step.am.txt"
    if grep -qE '^Error|Exception|unable to resolve' <<< "$output"; then
        save_outcome SKIP "the intent did not start the app: $(grep -m1 -E 'Error|Exception|unable' <<< "$output")"
        return 0
    fi
    deadline=$(($(date +%s) + OPEN_TIMEOUT))
    while :; do
        if ui_dump "$xml"; then
            if ui_query "$xml" contains "${FAILURE_TEXTS[@]}"; then
                shown=$(ui_query "$xml" show "${FAILURE_TEXTS[@]}" | head -n 1 || true)
                if ui_query "$xml" contains "${NO_PAGES_TEXTS[@]}"; then seen=1; fi
                break
            fi
            if ui_query "$xml" exact "${BACK_LABELS[@]}" && ui_query "$xml" "$TITLE_MODE" "${name%.*}" "$name"; then
                opened=1
                break
            fi
        fi
        alive || break
        [ "$(date +%s)" -lt "$deadline" ] || break
        sleep "$POLL"
    done
    # Anything written after the message (a copy, a repaired file) has time to show.
    sleep 3
    after=$(folder_list)
    added=$(added_files "$before" "$after")
    if ! original_kept "$name" "$old"; then
        save_outcome FAIL "$ORIGINAL_STATE"
    elif [ -n "$added" ]; then
        save_outcome FAIL "files appeared next to it: $added"
    elif [ "$opened" = 1 ]; then
        save_outcome FAIL "a document screen opened for a PDF without a readable page (an empty viewer, from which an empty PDF could be saved); $ORIGINAL_STATE"
    elif [ "$seen" = 1 ]; then
        save_outcome PASS "\"$shown\" shown, $ORIGINAL_STATE, nothing added to its folder"
    elif [ -n "$shown" ]; then
        save_outcome FAIL "opening failed for another reason: \"$shown\" (expected \"${NO_PAGES_TEXTS[0]}\"); $ORIGINAL_STATE"
    else
        save_outcome SKIP "neither the open failure nor a document screen was seen within $OPEN_TIMEOUT s; $ORIGINAL_STATE, nothing added to its folder"
    fi
}

# save_case NAME MIME ROUTE STEP CASE: cold start with the document, then the case's driver
# (block-pdf-open: the start itself is the case).
save_case() {
    local name=$1 mime=$2 route=$3 step=$4 case=$5 old mark found why
    S_RESULT=""
    S_DETAIL=""
    S_COPY="$OUT/$step.saved-$name"
    old=$(hash_of "$SAMPLES/$name")
    if [ "$case" != block-pdf-open ]; then
        open_document "$name" "$mime" "$route" viewer small "$step-open"
        if [ "$LAST_RESULT" != PASS ]; then
            record "$step" SKIP "the document did not open (see $step-open)"
            return
        fi
    fi
    mark=$(log_mark)
    # A driver leaves its outcome in S_RESULT and returns 0: under set -e any other status would
    # end the whole run.
    case "$case" in
        docx) save_word "$step" "$name" "$old" ;;
        xlsx) save_sheet "$step" "$name" "$old" ;;
        pptx) save_slides "$step" "$name" "$old" ;;
        txt-utf8 | txt-1254) save_text "$step" "$name" "$old" ;;
        block-xlsx) block_sheet "$step" "$name" "$old" ;;
        block-docx) block_word "$step" "$name" "$old" ;;
        block-pptx) block_slides "$step" "$name" "$old" ;;
        block-txt) block_text "$step" "$name" "$old" ;;
        pdf) save_pdf "$step" "$name" "$old" ;;
        block-pdf-open) refused_open "$step" "$name" "$mime" "$route" "$old" ;;
        *) save_outcome SKIP "unknown case \"$case\" in samples-save.tsv" ;;
    esac
    if [ "$S_RESULT" = WRITTEN ]; then
        if found=$(verify_saved "$case" "$SAMPLES/$name" "$S_COPY" 2>&1); then S_RESULT=PASS; else S_RESULT=FAIL; fi
        save_note "pulled file: $found"
    fi
    if ! alive; then
        S_RESULT=FAIL
        save_note "the app process is gone after saving"
    fi
    screenshot "$step"
    if ! check_logs "$step" "$mark"; then
        S_RESULT=FAIL
        save_note "crash signals in logcat (see the log above and $step.logcat.txt)"
    fi
    if [ "$S_RESULT" = FAIL ]; then
        case "$case" in
            docx | pptx | block-docx | block-pptx | pdf | block-pdf-open)
                why=$(app_log "$SAVE_LOG_TAG" "$name")
                [ -z "$why" ] || save_note "app log: $why"
                ;;
        esac
    fi
    if [ "$S_RESULT" = SKIP ]; then echo "::warning title=Emulator smoke test: $step skipped::$S_DETAIL"; fi
    record "$step" "${S_RESULT:-SKIP}" "${S_DETAIL:-the driver decided nothing}"
    sh_adb am force-stop "$PKG" || true
    if [ "$S_RESULT" = FAIL ]; then clear_crash_report; fi
}

save_pass() {
    local list="$SAMPLES/samples-save.tsv" elapsed name mime route step case
    # Shorter than the open-only passes: the edit that follows also gives a late crash time.
    local SETTLE=3
    PASS=3
    TITLE_MODE=title
    METRICS="$OUT/metrics-save.tsv"
    (IFS=$'\t'; columns=(step result "${M_COLUMNS[@]}"); printf '%s\n' "${columns[*]}") > "$METRICS"
    SAVE_STARTED=$(date +%s)
    elapsed=$(((SAVE_STARTED - RUN_STARTED) / 60))
    if [ ! -s "$list" ]; then
        record save-setup SKIP "no samples-save.tsv in $SAMPLES"
        return
    fi
    if [ "$elapsed" -ge $((SAVE_TIME_LIMIT - 4)) ]; then
        echo "::warning title=Emulator smoke test: save pass skipped::the earlier passes took $elapsed min (limit $SAVE_TIME_LIMIT min)"
        record save-setup SKIP "skipped: the earlier passes took $elapsed min, too close to the $SAVE_TIME_LIMIT min limit"
        return
    fi
    log "save pass: edit, save over the file, check it with adb"
    while IFS=$'\t' read -r name _ <&3; do
        timeout 60 adb push "$SAMPLES/$name" "$DEVICE_DIR/$name" > /dev/null < /dev/null
    done 3< "$list"
    while IFS=$'\t' read -r name mime route _ _ step case <&3; do
        if [ $((($(date +%s) - RUN_STARTED) / 60)) -ge "$SAVE_TIME_LIMIT" ]; then
            record "$step" SKIP "skipped: the run reached its $SAVE_TIME_LIMIT min limit"
            continue
        fi
        save_case "$name" "$mime" "$route" "$step" "$case"
    done 3< "$list"
    sh_adb am force-stop "$PKG" || true
    SAVE_ENDED=$(date +%s)
}

# --------------------------------------------------------------------------------------------
# Fourth pass: edit, stop the app without saving, reopen and restore or discard ("Kurtarma")

RECOVERY_STARTED=""
RECOVERY_ENDED=""
# What recovery_open saw: "prompt" (the recovery prompt), "document" (the document screen and no
# prompt), "failed" (an open failure, a dead process or nothing in time); R_DETAIL says more.
R_SEEN=""
R_DETAIL=""
# How long recovery_open keeps looking for the prompt once the document screen is up (a workbook
# shows it only after its import notes were read on the engine thread).
RECOVERY_GRACE=5

# recovery_lines MARK [TEXT]: the app's "BadoRecovery" lines in the recorded logcat since MARK
# (those containing TEXT).
recovery_lines() { tail -n +"$(($1 + 1))" "$LOGCAT" 2> /dev/null | grep -F " $RECOVERY_LOG_TAG: " | grep -F -- "${2:-}" || true; }

# recovery_open STEP NAME MIME ROUTE: a cold start with the document (a VIEW intent, as in
# open_document); the screen is watched until the recovery prompt shows or the document screen has
# been up for RECOVERY_GRACE s without one. Sets R_SEEN and R_DETAIL; import notes are read away
# once. Always status 0.
recovery_open() {
    local step=$1 name=$2 mime=$3 route=$4 xml="$OUT/$1.window.xml" uri output started deadline doc_at="" notes=0
    R_SEEN=""
    R_DETAIL=""
    uri="$URI_BASE/$(uri_encode "$name")"
    sh_adb am force-stop "$PKG" || true
    started=$(date +%s)
    if [ "$route" = filter ]; then
        output=$(sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -p "$PKG" 2>&1 || true)
    else
        output=$(sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -n "$ACTIVITY" 2>&1 || true)
    fi
    echo "$output" > "$OUT/$step.am.txt"
    if grep -qE '^Error|Exception|unable to resolve' <<< "$output"; then
        R_SEEN=failed
        R_DETAIL="the intent did not start the app: $(grep -m1 -E 'Error|Exception|unable' <<< "$output")"
        return 0
    fi
    deadline=$((started + OPEN_TIMEOUT))
    while :; do
        if ! alive; then
            R_SEEN=failed
            R_DETAIL="the app process died while opening"
            break
        fi
        if ui_dump "$xml"; then
            if ui_query "$xml" exact "${RESTORE_LABELS[@]}" && ui_query "$xml" contains "${RECOVERY_TITLES[@]}"; then
                R_SEEN=prompt
                R_DETAIL="recovery prompt shown after $(($(date +%s) - started)) s"
                break
            fi
            if ui_query "$xml" contains "${FAILURE_TEXTS[@]}"; then
                R_SEEN=failed
                R_DETAIL="the app reported a problem on screen: $(ui_query "$xml" show "${FAILURE_TEXTS[@]}" | head -3 | tr '\n' ' ')"
                break
            fi
            if [ "$notes" = 0 ] && ui_query "$xml" exact "${IMPORT_NOTES[@]}"; then
                sh_adb input keyevent KEYCODE_BACK || true
                notes=1
            elif [ -z "$doc_at" ] && ui_query "$xml" exact "${BACK_LABELS[@]}" && ui_query "$xml" "$TITLE_MODE" "${name%.*}" "$name"; then
                doc_at=$(date +%s)
            fi
        fi
        if [ -n "$doc_at" ] && [ "$(date +%s)" -ge $((doc_at + RECOVERY_GRACE)) ]; then
            R_SEEN=document
            R_DETAIL="document screen after $((doc_at - started)) s and no recovery prompt within $RECOVERY_GRACE s more"
            break
        fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            R_SEEN=failed
            R_DETAIL="neither the document screen nor the recovery prompt within $OPEN_TIMEOUT s (see $step.png / $step.window.xml)"
            break
        fi
        sleep "$POLL"
    done
    screenshot "$step"
}

# recovery_word_edit STEP: the save pass's Word edit, then (Back out of editing if needed) until the
# top bar shows "Save" (only a changed document has it). Status 1 (and a SKIP) when it does not.
recovery_word_edit() {
    local step=$1 xml="$OUT/$1.window.xml" i
    word_edit "$step" || return 1
    # The first Back may only close the keyboard; leaving editing brings Save to the top bar.
    for i in 1 2 3 4; do
        if ui_dump "$xml" && ui_query "$xml" exact "${SAVE_LABELS[@]}"; then
            save_note "typed \"$TYPED_WORD\" after \"Edit\", left editing: \"Save\" in the top bar"
            return 0
        fi
        if [ "$i" -lt 4 ]; then
            sh_adb input keyevent KEYCODE_BACK || true
            sleep 2
        fi
    done
    save_outcome SKIP "no \"Save\" in the top bar after typing \"$TYPED_WORD\" and leaving editing: the edit did not reach the document"
    return 1
}

# recovery_wait_copy MARK NAME: waits at least RECOVERY_WAIT s, and until the app logged the
# recovery copy of NAME as written (at most 25 s); notes which.
recovery_wait_copy() {
    local mark=$1 name=$2 started waited
    started=$(date +%s)
    while :; do
        waited=$(($(date +%s) - started))
        if [ "$waited" -ge "$RECOVERY_WAIT" ] && [ -n "$(recovery_lines "$mark" "copy written: $name")" ]; then
            save_note "recovery copy logged as written, app force-stopped after $waited s"
            return 0
        fi
        if [ "$waited" -ge 25 ]; then
            save_note "no \"copy written\" log line within $waited s of the edit, app force-stopped anyway"
            return 0
        fi
        sleep 1
    done
}

# recovery_restored_sheet STEP: after "Restore", the formula bar (A1 is selected again) must show
# the typed number; status 1 when it does not within 15 s. R_DETAIL says what was seen.
recovery_restored_sheet() {
    local step=$1 xml="$OUT/$1.window.xml" deadline shown="" bar="no \"Save\" in the top bar"
    deadline=$(($(date +%s) + 15))
    while :; do
        if ui_dump "$xml"; then
            shown=$(field_text "$step")
            if [ "$shown" = "$TYPED_NUMBER" ]; then
                if ui_query "$xml" exact "${SAVE_LABELS[@]}"; then bar="\"Save\" in the top bar"; fi
                R_DETAIL="the formula bar shows A1 = $shown again, $bar"
                return 0
            fi
        fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            R_DETAIL="the formula bar shows \"$shown\" after \"Restore\", not the typed $TYPED_NUMBER: the edit did not come back"
            return 1
        fi
        sleep 1
    done
}

# recovery_restored_word STEP: after "Restore" the top bar must show "Save" (status 1 when it does
# not within 15 s); then the viewer's search for the typed word sets W_FOUND to "yes", "no" or ""
# (the search could not be used). R_DETAIL says what was seen.
W_FOUND=""
recovery_restored_word() {
    local step=$1 xml="$OUT/$1.window.xml" at counter="" deadline
    W_FOUND=""
    if ! wait_label "$step" 15 exact "${SAVE_LABELS[@]}"; then
        R_DETAIL="no \"Save\" in the top bar within 15 s of \"Restore\": the copy was not opened"
        return 1
    fi
    R_DETAIL="\"Save\" in the top bar again"
    if ! tap_label "$step" center-exact "${SEARCH_LABELS[@]}"; then
        R_DETAIL="$R_DETAIL; no \"Search\" in the top bar, so the restored text was not checked"
        return 0
    fi
    sleep 1
    ui_dump "$xml" || true
    if ! at=$(ui_query "$xml" field-center); then
        R_DETAIL="$R_DETAIL; no search field after \"Search\", so the restored text was not checked"
        return 0
    fi
    tap_at "$at"
    sleep 1
    sh_adb input text "$TYPED_WORD"
    deadline=$(($(date +%s) + 15))
    while :; do
        sleep 1
        if ui_dump "$xml"; then
            # The counter beside the field ("1/1"); the top bar's page counter looks the same.
            counter=$(ui_query "$xml" after-field 2> /dev/null || true)
            if [[ "$counter" =~ ^[0-9]+/[0-9]+$ ]]; then
                W_FOUND=yes
                R_DETAIL="$R_DETAIL; the search finds \"$TYPED_WORD\" ($counter)"
                return 0
            fi
            if ui_query "$xml" exact "${NO_MATCH_TEXTS[@]}"; then
                W_FOUND=no
                R_DETAIL="$R_DETAIL; the search finds no \"$TYPED_WORD\": the restored document lacks the typed word"
                return 0
            fi
        fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            R_DETAIL="$R_DETAIL; the search showed no result within 15 s (beside the field: \"$counter\"), so the restored text was not checked"
            return 0
        fi
    done
}

# recovery_discarded STEP FIRST: after "Discard" the formula bar must show A1's old text FIRST and
# the top bar no "Save"; then a third cold start must offer nothing. Leaves its outcome in S_RESULT.
recovery_discarded() {
    local step=$1 first=$2 name=$3 mime=$4 route=$5 xml="$OUT/$1.window.xml" shown again_mark
    sleep 3
    ui_dump "$xml" || true
    shown=$(field_text "$step")
    if [ "$shown" = "$TYPED_NUMBER" ] || { [ -n "$first" ] && [ "$shown" != "$first" ]; }; then
        save_outcome FAIL "after \"Discard\" the formula bar shows \"$shown\", not A1's old \"$first\""
        return 0
    fi
    if ui_query "$xml" exact "${SAVE_LABELS[@]}"; then
        save_outcome FAIL "after \"Discard\" the top bar still offers \"Save\" (the workbook counts as changed)"
        return 0
    fi
    save_note "\"Discard\": A1 shows \"$shown\" again, no \"Save\""
    again_mark=$(log_mark)
    recovery_open "$step-again" "$name" "$mime" "$route"
    if [ "$R_SEEN" = prompt ]; then
        tap_label "$step" center-exact "${DISCARD_LABELS[@]}" || true
        save_outcome FAIL "the third cold start offered a recovery copy again: \"Discard\" did not delete it"
    elif [ "$R_SEEN" != document ]; then
        save_outcome FAIL "the third cold start failed: $R_DETAIL"
    elif [ -n "$(recovery_lines "$again_mark" "offered: $name")" ]; then
        save_outcome FAIL "the third cold start found a recovery copy (app log), though no prompt was seen"
    else
        save_outcome PASS "third cold start: $R_DETAIL"
    fi
}

# recovery_case NAME MIME ROUTE STEP CASE: cold start, the edit, the wait for the recovery copy,
# `am force-stop`, a cold start with the same file, then "Restore" or "Discard" and the checks.
recovery_case() {
    local name=$1 mime=$2 route=$3 step=$4 case=$5 xml="$OUT/$4.window.xml" old mark before after added first="" why
    S_RESULT=""
    S_DETAIL=""
    old=$(hash_of "$SAMPLES/$name")
    before=$(folder_list)
    mark=$(log_mark)
    recovery_open "$step-open" "$name" "$mime" "$route"
    if [ "$R_SEEN" = prompt ]; then
        # A copy left by an earlier run on this device: thrown away so the case starts clean.
        tap_label "$step" center-exact "${DISCARD_LABELS[@]}" || true
        save_note "a recovery copy from an earlier run was offered and discarded"
        if wait_label "$step" 15 exact "${BACK_LABELS[@]}"; then R_SEEN=document; fi
    fi
    if [ "$R_SEEN" != document ]; then
        save_outcome SKIP "the document did not open: $R_DETAIL"
    else
        case "$case" in
            recover-xlsx | discard-xlsx)
                ui_dump "$xml" || true
                first=$(field_text "$step")
                if sheet_edit "$step" && ! wait_label "$step" 10 exact "${SAVE_LABELS[@]}"; then
                    save_outcome SKIP "no \"Save\" in the top bar after the edit (field shows \"$(field_text "$step")\"): the edit was not taken"
                fi
                ;;
            recover-docx) recovery_word_edit "$step" || true ;;
            *) save_outcome SKIP "unknown case \"$case\" in samples-recovery.tsv" ;;
        esac
    fi
    if [ -z "$S_RESULT" ]; then
        recovery_wait_copy "$mark" "$name"
        # No onStop, no copy on leaving: what a crash or the system ending the app leaves behind.
        sh_adb am force-stop "$PKG" || true
        sleep 1
        if alive; then save_note "the process was still there after am force-stop"; fi
        recovery_open "$step-reopen" "$name" "$mime" "$route"
        if [ "$R_SEEN" = document ]; then
            save_outcome FAIL "reopened without saving: $R_DETAIL, so the unsaved edit is lost"
        elif [ "$R_SEEN" != prompt ]; then
            save_outcome FAIL "reopening failed: $R_DETAIL"
        elif [ "$case" = discard-xlsx ]; then
            save_note "$R_DETAIL"
            if tap_label "$step" center-exact "${DISCARD_LABELS[@]}"; then
                recovery_discarded "$step" "$first" "$name" "$mime" "$route"
            else
                save_outcome SKIP "the prompt's \"Discard\" could not be tapped"
            fi
        else
            save_note "$R_DETAIL"
            if ! tap_label "$step" center-exact "${RESTORE_LABELS[@]}"; then
                save_outcome SKIP "the prompt's \"Restore\" could not be tapped"
            elif [ "$case" = recover-xlsx ]; then
                if recovery_restored_sheet "$step"; then save_outcome PASS "\"Restore\": $R_DETAIL"; else save_outcome FAIL "$R_DETAIL"; fi
            elif ! recovery_restored_word "$step"; then
                save_outcome FAIL "$R_DETAIL"
            elif [ "$W_FOUND" = no ]; then
                save_outcome FAIL "\"Restore\": $R_DETAIL"
            else
                save_outcome PASS "\"Restore\": $R_DETAIL"
            fi
        fi
    fi
    # Whatever happened, nothing was saved: the opened file keeps its bytes and has no company.
    after=$(folder_list)
    added=$(added_files "$before" "$after")
    if ! original_kept "$name" "$old"; then
        S_RESULT=FAIL
        save_note "$ORIGINAL_STATE although nothing was saved"
    else
        save_note "$ORIGINAL_STATE"
    fi
    if [ -n "$added" ]; then
        S_RESULT=FAIL
        save_note "files appeared next to it: $added"
    fi
    screenshot "$step"
    if ! check_logs "$step" "$mark"; then
        S_RESULT=FAIL
        save_note "crash signals in logcat (see the log above and $step.logcat.txt)"
    fi
    if [ "$S_RESULT" = FAIL ]; then
        why=$(app_log "$RECOVERY_LOG_TAG" "$name")
        save_note "app log: ${why:-no $RECOVERY_LOG_TAG line for $name}"
    fi
    if [ "$S_RESULT" = SKIP ]; then echo "::warning title=Emulator smoke test: $step skipped::$S_DETAIL"; fi
    record "$step" "${S_RESULT:-SKIP}" "${S_DETAIL:-the driver decided nothing}"
    sh_adb am force-stop "$PKG" || true
    if [ "$S_RESULT" = FAIL ]; then clear_crash_report; fi
}

recovery_pass() {
    local list="$SAMPLES/samples-recovery.tsv" elapsed name mime route step case
    PASS=4
    TITLE_MODE=title
    RECOVERY_STARTED=$(date +%s)
    elapsed=$(((RECOVERY_STARTED - RUN_STARTED) / 60))
    if [ ! -s "$list" ]; then
        record recovery-setup SKIP "no samples-recovery.tsv in $SAMPLES"
        return
    fi
    if [ "$elapsed" -ge $((RECOVERY_TIME_LIMIT - 3)) ]; then
        echo "::warning title=Emulator smoke test: recovery pass skipped::the earlier passes took $elapsed min (limit $RECOVERY_TIME_LIMIT min)"
        record recovery-setup SKIP "skipped: the earlier passes took $elapsed min, too close to the $RECOVERY_TIME_LIMIT min limit"
        return
    fi
    log "recovery pass: edit, stop the app without saving, reopen, restore or discard"
    while IFS=$'\t' read -r name _ <&3; do
        timeout 60 adb push "$SAMPLES/$name" "$DEVICE_DIR/$name" > /dev/null < /dev/null
    done 3< "$list"
    while IFS=$'\t' read -r name mime route _ _ step case <&3; do
        if [ $((($(date +%s) - RUN_STARTED) / 60)) -ge "$RECOVERY_TIME_LIMIT" ]; then
            record "$step" SKIP "skipped: the run reached its $RECOVERY_TIME_LIMIT min limit"
            continue
        fi
        recovery_case "$name" "$mime" "$route" "$step" "$case"
    done 3< "$list"
    sh_adb am force-stop "$PKG" || true
    RECOVERY_ENDED=$(date +%s)
}

# --------------------------------------------------------------------------------------------
# Last pass: Google Play phone screenshots ("Mağaza ekran görüntüleri"). Cosmetic: it runs in a
# subshell whose errors are ignored, and a problem is a ::warning, never a failed step.

SHOTS_STORE="$SAMPLES/store"
SHOTS_DIR="$OUT/play-screenshots"
# One line per screenshot: locale, name, OK/SKIP/FAIL, detail (finish() puts it in the summary).
SHOTS_LOG="$OUT/play-screenshots.tsv"
SHOTS_DEVICE_DIR=/sdcard/Documents
SHOTS_URI_BASE=content://$PKG.files/storage/Documents
SHOTS_STARTED=""
SHOTS_ENDED=""

# shot_note LOCALE NAME RESULT DETAIL
shot_note() {
    printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" >> "$SHOTS_LOG"
    log "screenshot $1/$2: $3 - $4"
    [ "$3" = OK ] || echo "::warning title=Play screenshot $1/$2::$4"
}

shots_time_left() { [ $((($(date +%s) - RUN_STARTED) / 60)) -lt "$SHOTS_TIME_LIMIT" ]; }

# A path quoted for the device's shell (adb shell joins its arguments into one command line).
device_quote() { printf "'%s'" "${1//\'/\'\\\'\'}"; }

# png_check FILE: prints "WIDTHxHEIGHT" of a PNG; status 1 if it is not one, 2 if Play would refuse
# its size (each side 320-3840 px, the long side at most twice the short one).
png_check() {
    python3 - "$1" <<'PY'
import struct, sys
try:
    head = open(sys.argv[1], "rb").read(24)
except OSError:
    sys.exit(1)
if len(head) < 24 or head[:8] != b"\x89PNG\r\n\x1a\n" or head[12:16] != b"IHDR":
    sys.exit(1)
w, h = struct.unpack(">II", head[16:24])
print(f"{w}x{h}")
sys.exit(0 if 320 <= min(w, h) and max(w, h) <= 3840 and max(w, h) <= 2 * min(w, h) else 2)
PY
}

# shot_capture LOCALE NAME: the screen as LOCALE/NAME.png.
shot_capture() {
    local file="$SHOTS_DIR/$1/$2.png" size status=0
    timeout 30 adb exec-out screencap -p > "$file" 2> /dev/null < /dev/null || true
    size=$(png_check "$file") || status=$?
    case "$status" in
        0) shot_note "$1" "$2" OK "$size" ;;
        2) shot_note "$1" "$2" FAIL "$size is outside Play's limits (320-3840 px, at most 2:1); kept" ;;
        *) rm -f "$file"; shot_note "$1" "$2" FAIL "screencap gave no PNG" ;;
    esac
}

demo() { sh_adb am broadcast -a com.android.systemui.demo -e command "$@" > /dev/null 2>&1 || true; }

# System UI's demo mode: a fixed clock, full battery and signal, no notification icons.
demo_enter() {
    sh_adb settings put global sysui_demo_allowed 1 || true
    demo enter
    demo clock -e hhmm 1200
    demo battery -e level 100 -e plugged false -e powersave false
    demo network -e wifi show -e level 4 -e fully true
    demo network -e mobile show -e datatype none -e level 4 -e fully true
    demo notifications -e visible false
    demo status -e volume hide -e bluetooth hide -e location hide -e alarm hide -e sync hide -e mute hide -e speakerphone hide
}

# Puts back what shots_pass changed on the device (run in the main shell, whatever happened).
shots_cleanup() {
    log "screenshots: leaving demo mode, display size and density back, app locale follows the system"
    demo exit
    sh_adb settings put global sysui_demo_allowed 0 > /dev/null 2>&1 || true
    sh_adb wm size reset > /dev/null 2>&1 || true
    sh_adb wm density reset > /dev/null 2>&1 || true
    sh_adb cmd locale set-app-locales "$PKG" > /dev/null 2>&1 || true
    SHOTS_ENDED=$(date +%s)
}

# shot_document LOCALE NAME FILE MIME: cold start with FILE, then the screenshot once its document
# screen (Back and the file name in the title bar) is up and has settled. Status 1 if it never was.
shot_document() {
    local locale=$1 shot=$2 name=$3 mime=$4 step="shot-$1-$2" uri base deadline notes=0 xml
    xml="$OUT/$step.window.xml"
    base="${name%.*}"
    uri="$SHOTS_URI_BASE/$(uri_encode "$name")"
    sh_adb am force-stop "$PKG" || true
    sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -n "$ACTIVITY" > "$OUT/$step.am.txt" 2>&1 || true
    deadline=$(($(date +%s) + SHOTS_OPEN_TIMEOUT))
    while :; do
        if ui_dump "$xml"; then
            if ui_query "$xml" contains "${FAILURE_TEXTS[@]}"; then
                shot_note "$locale" "$shot" FAIL "the app reported a problem opening $name: $(ui_query "$xml" show "${FAILURE_TEXTS[@]}" | head -1)"
                return 1
            fi
            if [ "$notes" = 0 ] && ui_query "$xml" exact "${IMPORT_NOTES[@]}"; then
                sh_adb input keyevent KEYCODE_BACK || true
                notes=1
                sleep 1
                continue
            fi
            if ui_query "$xml" exact "${BACK_LABELS[@]}" && ui_query "$xml" title "$base" "$name"; then break; fi
        fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            shot_note "$locale" "$shot" FAIL "no document screen for $name within $SHOTS_OPEN_TIMEOUT s (on screen: $(ui_labels "$xml"))"
            return 1
        fi
        sleep 1
    done
    sleep "$SHOTS_SETTLE"
    shot_capture "$locale" "$shot"
}

# shot_home LOCALE FIRST_NAME: cold start of the home screen; the screenshot once it shows the
# language's own texts and FIRST_NAME in its list (or after 30 s, with a note).
shot_home() {
    local locale=$1 first="${2%.*}" step="shot-$1-01-home" xml deadline labels=("${HOME_EN[@]}") seen=0
    xml="$OUT/$step.window.xml"
    [ "${locale%%-*}" = tr ] && labels=("${HOME_TR[@]}")
    sh_adb am force-stop "$PKG" || true
    sh_adb am start -W -n "$ACTIVITY" > "$OUT/$step.am.txt" 2>&1 || true
    deadline=$(($(date +%s) + 30))
    while [ "$(date +%s)" -lt "$deadline" ]; do
        sleep 1
        if ui_dump "$xml" && ui_query "$xml" contains "${labels[@]}" && ui_query "$xml" contains "$first"; then
            seen=1
            break
        fi
    done
    sleep 2
    ui_dump "$xml" || true
    shot_capture "$locale" 01-home
    if [ "$seen" != 1 ]; then
        shot_note "$locale" 01-home SKIP "taken without seeing the language's home texts and \"$first\" in the list (on screen: $(ui_labels "$xml"))"
    elif ui_query "$xml" contains mo-smoke- mo-save- mo-recover- -duzenlendi; then
        shot_note "$locale" 01-home SKIP "the list still shows earlier passes' files: $(ui_query "$xml" show mo-smoke- mo-save- mo-recover- -duzenlendi | head -3 | tr '\n' ' ')"
    fi
}

# shots_locale LOCALE: every screenshot of one store language.
shots_locale() {
    local locale=$1 list="$SHOTS_STORE/shots.tsv" row_locale shot name mime first="" pushed=()
    mkdir -p "$SHOTS_DIR/$locale"
    # Only this language's files on the device: the home screen lists everything in shared storage.
    while IFS=$'\t' read -r row_locale _ name _ <&3; do
        sh_adb "rm -f $(device_quote "$SHOTS_DEVICE_DIR/$name")" > /dev/null 2>&1 || true
        if [ "$row_locale" = "$locale" ]; then
            timeout 60 adb push "$SHOTS_STORE/$row_locale/$name" "$SHOTS_DEVICE_DIR/$name" > /dev/null 2>&1 < /dev/null &&
                pushed+=("$name")
        fi
    done 3< "$list"
    log "screenshots $locale: ${#pushed[@]} files in $SHOTS_DEVICE_DIR"
    # No recent files or settings from the earlier passes; the language is the app's own.
    sh_adb pm clear "$PKG" > /dev/null 2>&1 || true
    sh_adb appops set "$PKG" MANAGE_EXTERNAL_STORAGE allow || true
    sh_adb cmd locale set-app-locales "$PKG" --locales "$locale" || true
    sleep 2
    demo_enter
    while IFS=$'\t' read -r row_locale shot name mime <&3; do
        [ "$row_locale" = "$locale" ] && [ "$shot" != "-" ] || continue
        [ -n "$first" ] || first=$name
        if ! shots_time_left; then
            shot_note "$locale" "$shot" SKIP "the run reached its $SHOTS_TIME_LIMIT min limit"
            continue
        fi
        if shot_document "$locale" "$shot" "$name" "$mime" && [ "$shot" = 05-pdf ]; then
            # The PDF stays open: its drawing tools are the optional sixth screenshot.
            if pdf_tool "shot-$locale-06-pdf-annotate" "${ANNOTATE_LABELS[@]}"; then
                sleep 2
                shot_capture "$locale" 06-pdf-annotate
            else
                shot_note "$locale" 06-pdf-annotate SKIP "no \"${ANNOTATE_LABELS[0]}\" tool found on the PDF screen"
            fi
        fi
    done 3< "$list"
    if shots_time_left; then
        shot_home "$locale" "$first"
    else
        shot_note "$locale" 01-home SKIP "the run reached its $SHOTS_TIME_LIMIT min limit"
    fi
    sh_adb am force-stop "$PKG" || true
}

# What the earlier passes left in shared storage, deleted and forgotten by MediaStore: the home
# screen lists every document MediaStore knows, and it indexes files pushed into the hidden smoke
# folder too (the first run's home screenshot listed mo-smoke-big-xlsx and "22 documents"). That
# folder (with the second pass's "RAPOR-İŞ.DOCX"-style copies), every "mo-smoke-", "mo-save-" and
# "mo-recover-" file anywhere, and the save pass's "…-duzenlendi" copies (Download) go; MediaStore
# rows left for them are deleted too. What MediaStore still lists is logged.
SAMPLE_ROWS="_data LIKE '%/.mo-smoke/%' OR _display_name LIKE 'mo-smoke-%' OR _display_name LIKE 'mo-save-%'"
SAMPLE_ROWS="$SAMPLE_ROWS OR _display_name LIKE 'mo-recover-%' OR _display_name LIKE '%-duzenlendi%'"
shots_forget_samples() {
    local left
    sh_adb rm -rf "$DEVICE_DIR" > /dev/null 2>&1 || true
    sh_adb "find /storage/emulated/0/ -type f \\( -name 'mo-smoke-*' -o -name 'mo-save-*' -o -name 'mo-recover-*' -o -name '*-duzenlendi*' \\) -delete" > /dev/null 2>&1 || true
    sh_adb content delete --uri content://media/external/file --where "\"$SAMPLE_ROWS\"" > /dev/null 2>&1 || true
    left=$(sh_adb content query --uri content://media/external/file --projection _data --where "\"$SAMPLE_ROWS\"" 2> /dev/null | grep -c '_data=' || true)
    if [ "${left:-0}" -gt 0 ]; then
        echo "::warning title=Play screenshots::MediaStore still lists $left of the earlier passes' files; the home screenshot may show them"
    fi
    log "screenshots: earlier passes' files deleted from shared storage and MediaStore (${left:-0} rows left)"
}

shots_pass() {
    local elapsed locale
    : > "$SHOTS_LOG"
    if [ ! -s "$SHOTS_STORE/shots.tsv" ]; then
        shot_note all setup SKIP "no store/shots.tsv in $SAMPLES (run scripts/make_store_samples.py $SAMPLES/store)"
        return 0
    fi
    elapsed=$((($(date +%s) - RUN_STARTED) / 60))
    if [ "$elapsed" -ge $((SHOTS_TIME_LIMIT - 3)) ]; then
        shot_note all setup SKIP "the earlier passes took $elapsed min, too close to the $SHOTS_TIME_LIMIT min limit"
        return 0
    fi
    log "screenshots: $SHOTS_LOCALES at $SHOTS_SIZE, $SHOTS_DENSITY dpi, font scale 1.0, light theme, demo status bar"
    mkdir -p "$SHOTS_DIR"
    sh_adb am force-stop "$PKG" || true
    shots_forget_samples
    sh_adb mkdir -p "$SHOTS_DEVICE_DIR" || true
    sh_adb settings put system font_scale 1.0 || true
    sh_adb cmd uimode night no > /dev/null || true
    sh_adb wm size "$SHOTS_SIZE" || true
    sh_adb wm density "$SHOTS_DENSITY" || true
    # System UI redraws for the new size and density before demo mode is set.
    sleep 4
    for locale in $SHOTS_LOCALES; do
        shots_locale "$locale"
    done
    # The last language's files are taken away again.
    while IFS=$'\t' read -r _ _ name _ <&3; do
        sh_adb "rm -f $(device_quote "$SHOTS_DEVICE_DIR/$name")" > /dev/null 2>&1 || true
    done 3< "$SHOTS_STORE/shots.tsv"
    log "screenshots: $(find "$SHOTS_DIR" -name '*.png' | wc -l) saved in $SHOTS_DIR"
}

finish() {
    local status=$?
    [ -n "${LOGCAT_PID:-}" ] && kill "$LOGCAT_PID" 2> /dev/null || true
    # Every buffer, including "crash", for the uploaded artifact.
    timeout 60 adb logcat -d -b all -v threadtime > "$OUT/logcat-all-buffers.txt" 2>&1 || true
    restore_device || true
    {
        echo "### Release APK on the emulator"
        echo
        echo "| Step | Result | am start | Visible | PSS | Detail |"
        echo "|---|---|---|---|---|---|"
        printf '%s\n' "${RESULTS[@]}"
        echo
        echo "*am start*: TotalTime of \`am start -W\` (cold start to the first frame). *Visible*: from the" \
            "intent until the UI poll saw the document (for the launch: the focused main screen), as" \
            "a range one poll wide. *PSS*: TOTAL PSS from \`dumpsys meminfo\` after ${SETTLE} s of settling." \
            "Soft budgets (warnings only): open ${OPEN_BUDGET} s, large files ${BIG_OPEN_BUDGET} s," \
            "PSS ${PSS_BUDGET_MB} MB; ${SLOW} over budget (⚠). Whole run: $((($(date +%s) - RUN_STARTED) / 60)) min." \
            "Device: $(head -1 "$OUT/device.txt" 2> /dev/null | tr -d '\r')."
        if [ "${#RESULTS_TR[@]}" -gt 0 ]; then
            echo
            echo "### tr-TR · yazı 2× · koyu"
            echo
            echo "| Step | Result | am start | Visible | PSS | Detail |"
            echo "|---|---|---|---|---|---|"
            printf '%s\n' "${RESULTS_TR[@]}"
            echo
            echo "The same checks with the app in Turkish (\`cmd locale set-app-locales $PKG --locales $TR_LOCALE\`)," \
                "font scale $TR_FONT_SCALE and the dark theme (\`cmd uimode night yes\`), on a subset of the formats" \
                "and on copies with upper-case extensions, Turkish letters and a space in their names; a document" \
                "screen passes only if its Back button says \"Geri\". ${SLOW_TR} over budget (⚠)." \
                "Pass: $(((${PASS2_ENDED:-$(date +%s)} - ${PASS2_STARTED:-$(date +%s)}) / 60)) min."
        fi
        if [ "${#RESULTS_SAVE[@]}" -gt 0 ]; then
            echo
            echo "### Kaydetme"
            echo
            echo "| Step | Result | am start | Visible | PSS | Detail |"
            echo "|---|---|---|---|---|---|"
            printf '%s\n' "${RESULTS_SAVE[@]}"
            echo
            echo "Edit and save over the file through the UI, then \`adb pull\` it and check it on the host" \
                "(*-open* rows: opening it, as above). *save-*: saving in place must write a valid file that holds" \
                "the edit and everything else (text files byte for byte, in their own encoding and line ends)." \
                "*save-pdf*: a filled-in form saved through \"Save as\" as a new *-duzenlendi.pdf*, the opened PDF unchanged." \
                "*block-*: files the app must not save over (a slicer, a VBA project in .xlsx; a damaged part in" \
                ".docx and an unreadable slide in .pptx, for which \"Save as\" must come up instead; mixed line ends)" \
                "or open (a PDF without a readable page); the file must keep its bytes. Pulled copies are in the logs artifact as *.saved-*. ${SLOW_SAVE} over budget (⚠)." \
                "Pass: $(((${SAVE_ENDED:-$(date +%s)} - ${SAVE_STARTED:-$(date +%s)}) / 60)) min."
        fi
        if [ "${#RESULTS_RECOVERY[@]}" -gt 0 ]; then
            echo
            echo "### Kurtarma"
            echo
            echo "| Step | Result | am start | Visible | PSS | Detail |"
            echo "|---|---|---|---|---|---|"
            printf '%s\n' "${RESULTS_RECOVERY[@]}"
            echo
            echo "Edit without saving, wait for the app's recovery copy (\`$RECOVERY_LOG_TAG\` \"copy written\" in logcat," \
                "at least ${RECOVERY_WAIT} s), end the app with \`am force-stop\` (no onStop: like a crash or the system" \
                "ending it), then open the same file again: the recovery prompt must come up. *recover-*: \"Restore\" must" \
                "bring the edit back (the formula bar shows the typed number; for Word, \"Save\" in the top bar and the" \
                "viewer's search finds the typed word). *discard-*: \"Discard\" must show the file as it is and the next" \
                "open must offer nothing. Every case: the opened file keeps its bytes and nothing appears next to it." \
                "Pass: $(((${RECOVERY_ENDED:-$(date +%s)} - ${RECOVERY_STARTED:-$(date +%s)}) / 60)) min."
        fi
        if [ -s "$SHOTS_LOG" ]; then
            echo
            echo "### Mağaza ekran görüntüleri"
            echo
            echo "| Locale | Screenshot | Result | Detail |"
            echo "|---|---|---|---|"
            awk -F '\t' '{ gsub(/\|/, "\\|", $4); printf "| %s | %s | %s | %s |\n", $1, $2, $3, $4 }' "$SHOTS_LOG"
            echo
            echo "Google Play phone screenshots (cosmetic: problems are warnings, never a failure), in the logs" \
                "artifact and the *play-screenshots* artifact as play-screenshots/<locale>/NN-name.png;" \
                "see docs/GOOGLE-PLAY.md. $(find "$SHOTS_DIR" -name '*.png' 2> /dev/null | wc -l) saved." \
                "Pass: $(((${SHOTS_ENDED:-$(date +%s)} - ${SHOTS_STARTED:-$(date +%s)}) / 60)) min."
        fi
    } > "$OUT/summary.md"
    cat "$OUT/summary.md"
    if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then cat "$OUT/summary.md" >> "$GITHUB_STEP_SUMMARY"; fi
    exit "$status"
}

# --------------------------------------------------------------------------------------------
# Main

[ -s "$APK" ] || { echo "APK not found: $APK" >&2; exit 2; }
[ -s "$SAMPLES/samples.tsv" ] || { echo "No samples.tsv in $SAMPLES (run scripts/make_smoke_samples.py)" >&2; exit 2; }

adb wait-for-device
for _ in $(seq 1 60); do
    [ "$(sh_adb getprop sys.boot_completed 2> /dev/null | tr -d '\r')" = 1 ] && break
    sleep 2
done
# Screen on, unlocked and staying on; no animations (uiautomator waits for an idle screen).
sh_adb input keyevent KEYCODE_WAKEUP || true
sh_adb wm dismiss-keyguard || true
sh_adb svc power stayon true || true
sh_adb settings put system screen_off_timeout 1800000 || true
for scale in window_animation_scale transition_animation_scale animator_duration_scale; do
    sh_adb settings put global "$scale" 0 || true
done
sh_adb getprop ro.build.fingerprint | tee "$OUT/device.txt"
sh_adb grep -E 'MemTotal|MemAvailable' /proc/meminfo | tee -a "$OUT/device.txt" || true
(IFS=$'\t'; columns=(step result "${M_COLUMNS[@]}"); printf '%s\n' "${columns[*]}") > "$METRICS"

adb logcat -c || true
adb logcat -v threadtime > "$LOGCAT" 2>&1 < /dev/null &
LOGCAT_PID=$!
trap finish EXIT

log "installing $APK"
if ! timeout 300 adb install -r -g "$APK" < /dev/null; then
    record install FAIL "adb install failed"
    exit 1
fi
# The release build must be the minified, non-debuggable one this test is about.
if sh_adb dumpsys package "$PKG" | grep -E '^\s*(pkgFlags|flags)=' | grep -q DEBUGGABLE; then
    record install FAIL "the installed package is debuggable: not the release build"
    exit 1
fi
sh_adb dumpsys package "$PKG" | grep -E 'versionName|versionCode' | head -2 || true
# "All files access", as granted in Settings (ADR-0002); the app reads the samples with it.
sh_adb appops set "$PKG" MANAGE_EXTERNAL_STORAGE allow
sh_adb appops get "$PKG" MANAGE_EXTERNAL_STORAGE || true
record install PASS "release APK installed, all-files access granted"

sh_adb rm -rf "$DEVICE_DIR"
sh_adb mkdir -p "$DEVICE_DIR"
sh_adb touch "$DEVICE_DIR/.nomedia"
while IFS=$'\t' read -r name _ _ _ <&3; do
    timeout 60 adb push "$SAMPLES/$name" "$DEVICE_DIR/$name" > /dev/null < /dev/null
done 3< "$SAMPLES/samples.tsv"
sh_adb ls -la "$DEVICE_DIR"

# The first start after installing, then a plain cold start for the baseline.
launch_home launch
launch_home launch-again
while IFS=$'\t' read -r name mime route expect size <&3; do
    open_document "$name" "$mime" "$route" "$expect" "${size:-small}"
done 3< "$SAMPLES/samples.tsv"
sh_adb am force-stop "$PKG" || true

if [ "$SECOND_PASS" = 1 ]; then
    second_pass
fi

# Last, in English again (second_pass put the app's locale back): it changes files.
if [ "$SAVE_PASS" = 1 ]; then
    save_pass
fi

# After saving, still in English: it leaves recovery copies in the app's own folder.
if [ "$RECOVERY_PASS" = 1 ]; then
    recovery_pass
fi

# Last and cosmetic: the Google Play screenshots. A subshell on the left of `||` runs without
# errexit, and nothing in it touches FAILED: whatever happens there, the run's result stays the same.
if [ "$SHOTS_PASS" = 1 ]; then
    SHOTS_STARTED=$(date +%s)
    (shots_pass) || echo "::warning title=Play screenshots::the screenshot pass stopped early (status $?)"
    shots_cleanup
fi

if [ "$FAILED" -gt 0 ]; then
    log "$FAILED step(s) failed"
    exit 1
fi
log "all steps passed"
