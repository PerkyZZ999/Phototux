#!/usr/bin/env python3
"""Check that every internal link in the handbook and the docs site resolves.

A cross-reference that lands on the wrong page is invisible: the browser scrolls
to the top of the file it did find and the reader assumes that is the section.
DR-024 was renamed from "Single document session v1" to "Document session model"
and five handbook pages went on pointing at the old anchor for months, quietly
sending anyone who followed them to the top of the Decision Register.

Two link kinds are checked:

* the file a relative link names must exist;
* the `#fragment` must match a heading on the page it lands on.

Heading slugs follow GitHub's rule — lowercase, drop everything that is not a
word character, a space or a hyphen, then turn *each* space into a hyphen. Runs
are not collapsed, which is why `## DR-023 — Tech stack` becomes
`dr-023--tech-stack`: the em dash is dropped and both spaces around it survive
as hyphens. Collapsing them is the mistake that makes this check report a
hundred false positives.

Usage:
    python3 scripts/check-docs-links.py          # handbook + docs site
    python3 scripts/check-docs-links.py --quiet  # exit code only
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

# The handbook is a tree of files a reader opens directly, so its links are
# relative paths. The docs site is Astro: its links are *routes* — `/guides/
# layers/` is `guides/layers.md` — so the two roots resolve differently and a
# checker that treats them the same reports every site link as broken.
FILE_ROOTS = ["internal_docs"]
ROUTE_ROOTS = ["web/docs/src/content/docs"]
LINK = re.compile(r"\]\(([^)\s]*?)(#[^)\s]*)?\)")
HEADING = re.compile(r"^#{1,6}\s+(.+)$", re.M)
EXTERNAL = ("http://", "https://", "mailto:", "tel:")


def slugify(text: str) -> str:
    """The anchor GitHub gives a heading."""
    text = re.sub(r"`([^`]*)`", r"\1", text)
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = text.strip().lower()
    text = re.sub(r"[^\w\s-]", "", text, flags=re.UNICODE)
    return text.replace(" ", "-")


def headings(path: pathlib.Path) -> set[str]:
    try:
        return {slugify(m.group(1)) for m in HEADING.finditer(path.read_text())}
    except OSError:
        return set()


def route_of(page: pathlib.Path, root: pathlib.Path) -> str:
    """The URL Astro serves this page at."""
    slug = str(page.relative_to(root)).removesuffix(".md")
    return "/" if slug == "index" else f"/{slug}/"


def check_routes(root: pathlib.Path) -> list[str]:
    """Links in the Astro site, which are routes rather than file paths."""
    pages = sorted(root.rglob("*.md"))
    routes = {route_of(p, root): p for p in pages}
    anchors = {route_of(p, root): headings(p) for p in pages}
    # Files served straight from `public/`, which are not pages.
    static = root.parent.parent.parent / "public"
    problems: list[str] = []
    for page in pages:
        text = page.read_text()
        for match in LINK.finditer(text):
            href, fragment = match.group(1), match.group(2)
            if href.startswith(EXTERNAL) or not href.startswith("/"):
                continue
            line = text[: match.start()].count("\n") + 1
            where = f"{page}:{line}"
            if "." in href.rsplit("/", 1)[-1]:
                if not (static / href.lstrip("/")).exists():
                    problems.append(f"{where}: no such asset — {href}")
                continue
            route = href if href.endswith("/") else href + "/"
            if route not in routes:
                problems.append(f"{where}: no such page — {href}")
                continue
            if fragment and fragment[1:] not in anchors[route]:
                problems.append(f"{where}: no such heading — {href}{fragment}")
    return problems


def check(root: pathlib.Path) -> list[str]:
    pages = sorted(root.rglob("*.md"))
    anchors = {p.resolve(): headings(p) for p in pages}
    problems: list[str] = []
    for page in pages:
        text = page.read_text()
        for match in LINK.finditer(text):
            href, fragment = match.group(1), match.group(2)
            if href.startswith(EXTERNAL):
                continue
            line = text[: match.start()].count("\n") + 1
            where = f"{page}:{line}"
            # A bare `#anchor` points at the page it is written on.
            target = page.resolve() if not href else (page.parent / href).resolve()
            if href and not target.exists():
                problems.append(f"{where}: no such file — {href}")
                continue
            if fragment:
                known = anchors.get(target)
                # Only pages this script parsed have a heading set; a link into
                # a non-markdown file carries no anchor this can verify.
                if known is not None and fragment[1:] not in known:
                    problems.append(f"{where}: no such heading — {href}{fragment}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--quiet", action="store_true", help="exit code only")
    args = parser.parse_args()

    problems: list[str] = []
    checked = 0
    for name in FILE_ROOTS:
        root = pathlib.Path(name)
        if not root.is_dir():
            continue
        checked += len(list(root.rglob("*.md")))
        problems.extend(check(root))
    for name in ROUTE_ROOTS:
        root = pathlib.Path(name)
        if not root.is_dir():
            continue
        checked += len(list(root.rglob("*.md")))
        problems.extend(check_routes(root))

    if problems:
        if not args.quiet:
            print(f"{len(problems)} broken link(s) in {checked} pages:\n")
            for problem in problems:
                print(f"  {problem}")
        return 1
    if not args.quiet:
        print(f"all internal links resolve ({checked} pages)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
