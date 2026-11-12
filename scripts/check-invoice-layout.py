#!/usr/bin/env python3
"""Check exported invoice HTML in Chromium (requires Python Playwright and Chromium).

MO_INVOICE_HTML_DIR=/tmp/mo-invoice ./gradlew testDebugUnitTest --tests '*InvoiceTest' --rerun-tasks
python scripts/check-invoice-layout.py /tmp/mo-invoice
"""
import argparse
import pathlib
import shutil
from playwright.sync_api import sync_playwright


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=pathlib.Path)
    args = parser.parse_args()
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=shutil.which("chromium"), args=["--no-sandbox"])
        try:
            for mobile in (False, True):
                context = browser.new_context(viewport={"width": 390 if mobile else 1200, "height": 844}, is_mobile=mobile)
                try:
                    for name in ("seller", "fragment", "fallback"):
                        page = context.new_page()
                        try:
                            page.set_content((args.directory / f"{name}.html").read_text())
                            width = page.evaluate("parseFloat(getComputedStyle(document.body).width)")
                            assert abs(width - 794) < 1, (name, mobile, "screen width", width)
                            assert page.evaluate("document.querySelectorAll('meta[name=viewport]').length") == 1
                            if name == "seller":
                                page.emulate_media(media="print")
                                style = page.evaluate("({width:parseFloat(getComputedStyle(document.body).width),margin:parseFloat(getComputedStyle(document.body).marginTop),desk:getComputedStyle(document.documentElement).backgroundColor})")
                                assert style["width"] == 640, ("seller print width", style)
                                assert style["margin"] == 7, ("seller print margin", style)
                                assert style["desk"] != "rgb(214, 217, 220)", ("desk in print", style)
                            checks = "screen and seller print styles" if name == "seller" else "screen styles"
                            print(f"OK {name}: {'mobile' if mobile else 'desktop'} {checks}")
                        finally:
                            page.close()
                finally:
                    context.close()
        finally:
            browser.close()


if __name__ == "__main__":
    main()
