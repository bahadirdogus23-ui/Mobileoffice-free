#!/usr/bin/env python3
"""Render the Google Play store graphics from the app's launcher icon.

Reads the adaptive launcher icon (android/app/src/main/res/drawable/ic_launcher_background.xml
and ic_launcher_foreground.xml), converts the Android drawables to SVG and renders:

  fastlane/metadata/android/<locale>/images/icon.png            512x512, 32-bit RGBA
  fastlane/metadata/android/<locale>/images/featureGraphic.png  1024x500, RGB, no alpha

for the locales in LOCALES. Rendering uses headless Chromium through Playwright for Node
(`require('playwright')`), with the locally installed DejaVu Sans font. Nothing is downloaded.
If Pillow is installed, the PNGs are re-encoded with optimisation (and alpha removed from the
feature graphic).

Usage: python3 scripts/store-graphics.py [--keep-svg DIR]
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent
RES = ROOT / "android" / "app" / "src" / "main" / "res"
FASTLANE = ROOT / "fastlane" / "metadata" / "android"
ANDROID_NS = "{http://schemas.android.com/apk/res/android}"

APP_NAME = "BadoOffice"
LOCALES = {
    "tr": {"lang": "tr", "tagline": "Ücretsiz, reklamsız, internetsiz ofis"},
    "en-US": {"lang": "en", "tagline": "Free, ad-free, offline office suite"},
}
FONT_DIRS = ["/usr/share/fonts/truetype/dejavu", "/usr/share/fonts/dejavu"]


def a(el: ET.Element, name: str, default: str | None = None) -> str | None:
    return el.get(ANDROID_NS + name, default)


def android_color(value: str) -> tuple[str, float]:
    """Convert #RGB/#ARGB/#RRGGBB/#AARRGGBB to (#RRGGBB, opacity)."""
    v = value.strip().lstrip("#")
    if len(v) in (3, 4):
        v = "".join(c * 2 for c in v)
    if len(v) == 8:
        return "#" + v[2:], int(v[:2], 16) / 255
    if len(v) == 6:
        return "#" + v, 1.0
    raise ValueError(f"unsupported colour {value!r}")


def paint(attr: str, value: str | None, alpha: str | None) -> str:
    if not value:
        return f'{attr}="none"'
    if value.startswith("@"):
        raise ValueError(f"colour resources are not supported: {value}")
    color, opacity = android_color(value)
    if alpha is not None:
        opacity *= float(alpha)
    out = f'{attr}="{color}"'
    if opacity < 1:
        out += f' {attr}-opacity="{opacity:.3f}"'
    return out


def vector_children(el: ET.Element) -> str:
    """Convert the children of a <vector> or <group> to SVG elements."""
    parts = []
    for child in el:
        tag = child.tag
        if tag == "path":
            attrs = [f'd="{escape(a(child, "pathData", ""))}"']
            attrs.append(paint("fill", a(child, "fillColor"), a(child, "fillAlpha")))
            stroke = a(child, "strokeColor")
            if stroke:
                attrs.append(paint("stroke", stroke, a(child, "strokeAlpha")))
                attrs.append(f'stroke-width="{a(child, "strokeWidth", "0")}"')
                attrs.append(f'stroke-linecap="{a(child, "strokeLineCap", "butt")}"')
                attrs.append(f'stroke-linejoin="{a(child, "strokeLineJoin", "miter")}"')
            if a(child, "fillType") == "evenOdd":
                attrs.append('fill-rule="evenodd"')
            parts.append(f"<path {' '.join(attrs)}/>")
        elif tag == "group":
            px, py = float(a(child, "pivotX", "0")), float(a(child, "pivotY", "0"))
            tx, ty = a(child, "translateX", "0"), a(child, "translateY", "0")
            sx, sy = a(child, "scaleX", "1"), a(child, "scaleY", "1")
            rot = a(child, "rotation", "0")
            transform = (
                f"translate({tx} {ty}) translate({px} {py}) rotate({rot}) "
                f"scale({sx} {sy}) translate({-px} {-py})"
            )
            parts.append(f'<g transform="{transform}">{vector_children(child)}</g>')
        elif tag == "clip-path":
            raise ValueError("clip-path is not supported; extend store-graphics.py")
    return "".join(parts)


def drawable_to_svg_group(path: Path, size: float) -> str:
    """Return an SVG <g> drawing the drawable into a size x size box at the origin."""
    root = ET.parse(path).getroot()
    if root.tag == "vector":
        vw = float(a(root, "viewportWidth"))
        vh = float(a(root, "viewportHeight"))
        return (
            f'<g transform="scale({size / vw} {size / vh})">{vector_children(root)}</g>'
        )
    if root.tag == "shape" and a(root, "shape", "rectangle") == "rectangle":
        solid = root.find("solid")
        if solid is None:
            raise ValueError(f"{path}: only <solid> shapes are supported")
        return f'<rect width="{size}" height="{size}" {paint("fill", a(solid, "color"), None)}/>'
    raise ValueError(f"{path}: unsupported drawable <{root.tag}>")


def launcher_layers() -> tuple[Path, Path]:
    icon = ET.parse(RES / "mipmap-anydpi" / "ic_launcher.xml").getroot()
    layers = {}
    for name in ("background", "foreground"):
        ref = a(icon.find(name), "drawable")
        if not ref or not ref.startswith("@drawable/"):
            raise ValueError(f"unsupported {name} reference {ref!r}")
        layers[name] = RES / "drawable" / (ref.split("/", 1)[1] + ".xml")
    return layers["background"], layers["foreground"]


def background_color(path: Path) -> str:
    solid = ET.parse(path).getroot().find("solid")
    return android_color(a(solid, "color"))[0] if solid is not None else "#1F5FAD"


def shade(color: str, factor: float) -> str:
    r, g, b = (int(color[i : i + 2], 16) for i in (1, 3, 5))
    return "#" + "".join(f"{round(c * factor):02X}" for c in (r, g, b))


def font_face() -> str:
    faces = []
    for weight, name in (("400", "DejaVuSans.ttf"), ("700", "DejaVuSans-Bold.ttf")):
        for d in FONT_DIRS:
            f = Path(d) / name
            if f.exists():
                faces.append(
                    f"@font-face{{font-family:'StoreFont';font-weight:{weight};"
                    f"src:url('{f.as_uri()}');}}"
                )
                break
        else:
            sys.exit(f"font {name} not found in {FONT_DIRS}; install fonts-dejavu-core")
    return "".join(faces)


def icon_svg(bg: Path, fg: Path, size: int = 512) -> str:
    # Play applies its own mask and shadow: a full-bleed square is what it expects.
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" '
        f'viewBox="0 0 {size} {size}">'
        f"{drawable_to_svg_group(bg, size)}{drawable_to_svg_group(fg, size)}</svg>"
    )


def feature_svg(bg: Path, fg: Path, tagline: str, lang: str) -> str:
    w, h = 1024, 500
    base = background_color(bg)
    dark = shade(base, 0.78)  # backdrop a little darker than the icon so the tile stands out
    tile, tx, ty = 300, 88, 100  # the icon, as a rounded tile like on a home screen
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"
 viewBox="0 0 {w} {h}" lang="{lang}">
 <defs>
  <style>{font_face()}text{{font-family:'StoreFont',sans-serif}}</style>
  <clipPath id="tile"><rect x="{tx}" y="{ty}" width="{tile}" height="{tile}" rx="{tile * 0.22}"/></clipPath>
  <filter id="shadow" x="-20%" y="-20%" width="140%" height="150%">
   <feDropShadow dx="0" dy="10" stdDeviation="14" flood-color="#06264D" flood-opacity="0.35"/>
  </filter>
 </defs>
 <rect width="{w}" height="{h}" fill="{dark}"/>
 <circle cx="{w - 40}" cy="-40" r="300" fill="#FFFFFF" fill-opacity="0.06"/>
 <circle cx="{w - 120}" cy="{h + 120}" r="220" fill="#FFFFFF" fill-opacity="0.05"/>
 <g filter="url(#shadow)">
  <rect x="{tx}" y="{ty}" width="{tile}" height="{tile}" rx="{tile * 0.22}" fill="{base}"/>
 </g>
 <g clip-path="url(#tile)">
  <g transform="translate({tx} {ty})">{drawable_to_svg_group(bg, tile)}{drawable_to_svg_group(fg, tile)}</g>
 </g>
 <rect x="{tx + 0.5}" y="{ty + 0.5}" width="{tile - 1}" height="{tile - 1}" rx="{tile * 0.22}"
  fill="none" stroke="#FFFFFF" stroke-opacity="0.35" stroke-width="1"/>
 <text x="440" y="238" font-size="84" font-weight="700" fill="#FFFFFF"
  data-fit="500">{escape(APP_NAME)}</text>
 <rect x="443" y="270" width="64" height="6" rx="3" fill="#BBD3F2"/>
 <text x="442" y="330" font-size="29" font-weight="400" fill="#E3EDFA"
  data-fit="500">{escape(tagline)}</text>
</svg>"""


NODE_RENDER = r"""
const { chromium } = require('playwright');
const jobs = JSON.parse(process.argv[1]);
(async () => {
  const browser = await chromium.launch();
  try {
    for (const job of jobs) {
      const page = await browser.newPage({
        viewport: { width: job.width, height: job.height }, deviceScaleFactor: 1 });
      await page.goto(job.html);
      await page.evaluate(() => document.fonts.ready);
      // Shrink any text marked data-fit until it fits that width (no clipping).
      await page.evaluate(() => {
        for (const t of document.querySelectorAll('text[data-fit]')) {
          const max = Number(t.dataset.fit);
          let size = parseFloat(t.getAttribute('font-size'));
          while (t.getComputedTextLength() > max && size > 10) {
            size -= 0.5; t.setAttribute('font-size', String(size));
          }
        }
      });
      await page.screenshot({ path: job.out, omitBackground: job.alpha,
        clip: { x: 0, y: 0, width: job.width, height: job.height } });
      await page.close();
    }
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exit(1); });
"""


def node_module_path() -> str:
    paths = [os.environ.get("NODE_PATH", "")]
    try:
        root = subprocess.run(["npm", "root", "-g"], capture_output=True, text=True).stdout
        paths.append(root.strip())
    except OSError:
        pass
    paths += ["/opt/node-tools/node_modules", "/usr/lib/node_modules", "/usr/local/lib/node_modules"]
    return os.pathsep.join(p for p in paths if p)


def optimise(png: Path, keep_alpha: bool) -> None:
    try:
        from PIL import Image
    except ImportError:
        return
    with Image.open(png) as im:
        im = im.convert("RGBA" if keep_alpha else "RGB")
        im.save(png, optimize=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--keep-svg", type=Path, help="also write the SVG sources here")
    args = parser.parse_args()

    if not shutil.which("node"):
        sys.exit("node is required (with the playwright package and its Chromium)")
    bg, fg = launcher_layers()

    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        jobs = []

        def add(name: str, svg: str, width: int, height: int, out: Path, alpha: bool) -> None:
            if args.keep_svg:
                args.keep_svg.mkdir(parents=True, exist_ok=True)
                (args.keep_svg / f"{name}.svg").write_text(svg, encoding="utf-8")
            html = tmpdir / f"{name}.html"
            html.write_text(
                '<!doctype html><meta charset="utf-8">'
                "<style>html,body{margin:0;background:transparent}svg{display:block}</style>"
                + svg,
                encoding="utf-8",
            )
            out.parent.mkdir(parents=True, exist_ok=True)
            jobs.append(
                {"html": html.as_uri(), "out": str(out), "width": width, "height": height,
                 "alpha": alpha}
            )

        icon = icon_svg(bg, fg)
        for locale, info in LOCALES.items():
            images = FASTLANE / locale / "images"
            add(f"icon-{locale}", icon, 512, 512, images / "icon.png", True)
            add(f"feature-{locale}", feature_svg(bg, fg, info["tagline"], info["lang"]),
                1024, 500, images / "featureGraphic.png", False)

        env = dict(os.environ, NODE_PATH=node_module_path())
        subprocess.run(["node", "-e", NODE_RENDER, json.dumps(jobs)], check=True, env=env)

    for job in jobs:
        out = Path(job["out"])
        optimise(out, keep_alpha=job["alpha"])
        print(f"{out.relative_to(ROOT)}  {out.stat().st_size // 1024} KiB")


if __name__ == "__main__":
    main()
