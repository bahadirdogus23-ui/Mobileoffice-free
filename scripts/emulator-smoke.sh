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
# screen is checked, not just logcat.
#
# Environment: SMOKE_OPEN_TIMEOUT (s, default 60), SMOKE_SETTLE (s, default 6), SMOKE_OCR (1/0,
# default 1: run text recognition on the picture), SMOKE_LOG_IGNORE (extended regex of logcat lines
# to disregard, for a known harmless message).

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
SETTLE=${SMOKE_SETTLE:-6}
OCR=${SMOKE_OCR:-1}
LOG_IGNORE=${SMOKE_LOG_IGNORE:-}

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

# Logcat lines from the app's own process that mean a crash or a build that R8 broke.
PID_PATTERNS='FATAL EXCEPTION|JNI DETECTED ERROR|UnsatisfiedLinkError|ClassNotFoundException|NoClassDefFoundError|NoSuchMethodError|NoSuchFieldError|ExceptionInInitializerError|AbstractMethodError|VerifyError|Fatal signal|Abort message'
# Lines any process prints about the app (the system server reports ANRs, debuggerd native crashes).
PKG_RE='org\.mobileofficefree\.app'
GLOBAL_PATTERNS="Process: $PKG_RE, PID|ANR in $PKG_RE|>>> $PKG_RE <<<"

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
LOGCAT="$OUT/logcat.txt"
RESULTS=()
FAILED=0

log() { printf '[smoke %s] %s\n' "$(date +%H:%M:%S)" "$*"; }

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
# "exact" (equals a label, or one comma-separated part of a merged node does), "center" (prints
# "x y" of the first node containing a label) or "show" (prints every value containing a label).
# Exit status 0 when found.
ui_query() {
    python3 - "$@" << 'PY'
import re
import sys
import xml.etree.ElementTree as ET

path, mode, labels = sys.argv[1], sys.argv[2], sys.argv[3:]
found = False
try:
    nodes = list(ET.parse(path).getroot().iter("node"))
except (ET.ParseError, OSError):
    sys.exit(1)
for node in nodes:
    for value in (node.get("text") or "", node.get("content-desc") or ""):
        if not value:
            continue
        if mode == "exact":
            hit = any(value == l or l in [p.strip() for p in value.split(",")] for l in labels)
        else:
            hit = any(l in value for l in labels)
        if hit:
            if mode == "show":
                print(value[:300])
                found = True
                continue
            if mode == "center":
                x1, y1, x2, y2 = map(int, re.findall(r"\d+", node.get("bounds", "[0,0][0,0]")))
                print((x1 + x2) // 2, (y1 + y2) // 2)
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

record() {  # record STEP RESULT DETAIL
    RESULTS+=("| $1 | $2 | ${3//|//} |")
    if [ "$2" = "FAIL" ]; then
        FAILED=$((FAILED + 1))
        echo "::error title=Emulator smoke test: $1::$3"
    fi
    log "$1: $2 - $3"
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

# open_document NAME MIME ROUTE EXPECT: cold start with a VIEW intent for one sample.
open_document() {
    local name=$1 mime=$2 route=$3 expect=$4
    local base="${name%.*}"
    local step="open-${base#mo-smoke-}" uri="$URI_BASE/$name"
    local xml="$OUT/$step.window.xml" mark started output deadline outcome="" detail="" notes_dismissed=0
    sh_adb am force-stop "$PKG" || true
    mark=$(log_mark)
    started=$(date +%s)
    if [ "$route" = filter ]; then
        # Implicit, limited to the app: the manifest's intent filter has to accept the type.
        output=$(sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -p "$PKG" 2>&1 || true)
    else
        output=$(sh_adb am start -W -a android.intent.action.VIEW -d "$uri" -t "$mime" -n "$ACTIVITY" 2>&1 || true)
    fi
    echo "$output" > "$OUT/$step.am.txt"
    if grep -qE '^Error|Exception|unable to resolve' <<< "$output"; then
        outcome=FAIL
        detail="the intent did not start the app: $(grep -m1 -E 'Error|Exception|unable' <<< "$output")"
    fi
    deadline=$((started + OPEN_TIMEOUT))
    while [ -z "$outcome" ]; do
        if ! alive; then
            outcome=FAIL
            detail="the app process died while opening"
            break
        fi
        if ui_dump "$xml"; then
            if ui_query "$xml" contains "${FAILURE_TEXTS[@]}"; then
                outcome=FAIL
                detail="the app reported a problem on screen: $(ui_query "$xml" show "${FAILURE_TEXTS[@]}" | head -3 | tr '\n' ' ')"
                break
            fi
            if ui_query "$xml" exact "${BACK_LABELS[@]}" && ui_query "$xml" exact "$base" "$name"; then
                outcome=PASS
                detail="document screen \"$base\" shown after $(($(date +%s) - started)) s"
                break
            fi
            if [ "$expect" = viewer-or-notice ] && ui_query "$xml" contains "${NOTICE_TEXTS[@]}"; then
                outcome=PASS
                detail="the app announced the format as upcoming (no viewer yet)"
                break
            fi
            # Import notes are shown over the document after it opened: read them away once.
            if [ "$notes_dismissed" = 0 ] && ui_query "$xml" exact "${IMPORT_NOTES[@]}"; then
                log "$step: import notes shown, dismissing them"
                sh_adb input keyevent KEYCODE_BACK || true
                notes_dismissed=1
            fi
        fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            outcome=FAIL
            detail="no document screen for \"$base\" within $OPEN_TIMEOUT s (see $step.png / $step.window.xml)"
            break
        fi
        sleep 2
    done
    if [ "$outcome" = PASS ]; then
        # Pages render and engines keep working after the first frame: give a late crash time.
        sleep "$SETTLE"
        if ! alive; then
            outcome=FAIL
            detail="the app process died within $SETTLE s after the document appeared"
        fi
    fi
    screenshot "$step"
    if ! check_logs "$step" "$mark"; then
        outcome=FAIL
        detail="${detail}; crash signals in logcat (see the log above and $step.logcat.txt)"
    fi
    record "$step" "$outcome" "$detail"
    if [ "$outcome" = PASS ] && [ "$name" = mo-smoke-png.png ] && [ "$OCR" = 1 ]; then
        recognise_text
    fi
    if [ "$outcome" = FAIL ]; then clear_crash_report; fi
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

launch_home() {
    local step=launch xml="$OUT/launch.window.xml" mark i outcome=FAIL detail="the main screen did not come up within 30 s"
    sh_adb am force-stop "$PKG" || true
    mark=$(log_mark)
    sh_adb am start -W -n "$ACTIVITY" > "$OUT/launch.am.txt" 2>&1 || true
    for i in $(seq 1 15); do
        sleep 2
        alive || { detail="the app process died at start"; break; }
        if sh_adb dumpsys window 2> /dev/null | grep -E 'mCurrentFocus|mFocusedApp' | grep -q "$PKG"; then
            outcome=PASS
            detail="main screen focused after $((i * 2)) s"
            break
        fi
    done
    if [ "$outcome" = PASS ]; then
        sleep "$SETTLE"
        alive || { outcome=FAIL; detail="the app process died within $SETTLE s after start"; }
        if ui_dump "$xml" && ui_query "$xml" contains "${FAILURE_TEXTS[@]}"; then
            outcome=FAIL
            detail="the main screen shows a problem message"
        fi
    fi
    screenshot "$step"
    check_logs "$step" "$mark" || { outcome=FAIL; detail="${detail}; crash signals in logcat"; }
    record "$step" "$outcome" "$detail"
}

finish() {
    local status=$?
    [ -n "${LOGCAT_PID:-}" ] && kill "$LOGCAT_PID" 2> /dev/null || true
    # Every buffer, including "crash", for the uploaded artifact.
    timeout 60 adb logcat -d -b all -v threadtime > "$OUT/logcat-all-buffers.txt" 2>&1 || true
    {
        echo "### Release APK on the emulator"
        echo
        echo "| Step | Result | Detail |"
        echo "|---|---|---|"
        printf '%s\n' "${RESULTS[@]}"
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

launch_home
while IFS=$'\t' read -r name mime route expect <&3; do
    open_document "$name" "$mime" "$route" "$expect"
done 3< "$SAMPLES/samples.tsv"
sh_adb am force-stop "$PKG" || true

if [ "$FAILED" -gt 0 ]; then
    log "$FAILED step(s) failed"
    exit 1
fi
log "all steps passed"
