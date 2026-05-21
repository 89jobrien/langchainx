"""Wiki page management: CRUD, search, index maintenance, lint.

Pages are stored as markdown files under wiki/<category>/<slug>.md.
Index and log are maintained automatically.
"""

import os
import re
from datetime import datetime, timezone
from pathlib import Path

WIKI_DIR = Path(__file__).resolve().parent.parent / "wiki"
INDEX_FILE = WIKI_DIR / "index.md"
LOG_FILE = WIKI_DIR / "log.md"


def _slugify(title: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")


def _log(action: str, category: str, title: str):
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M")
    entry = f"- [{now}] {action}: {category}/{title}\n"
    with open(LOG_FILE, "a") as f:
        f.write(entry)


def write_page(
    category: str,
    title: str,
    content: str,
    summary: str = "",
) -> str:
    """Write or overwrite a wiki page. Returns the file path."""
    cat_dir = WIKI_DIR / category
    cat_dir.mkdir(parents=True, exist_ok=True)
    slug = _slugify(title)
    page_path = cat_dir / f"{slug}.md"
    page_path.write_text(content)
    _log("write", category, title)
    _update_index()
    return str(page_path)


def read_page(category: str, title: str) -> str | None:
    slug = _slugify(title)
    page_path = WIKI_DIR / category / f"{slug}.md"
    if page_path.exists():
        return page_path.read_text()
    return None


def delete_page(category: str, title: str) -> bool:
    slug = _slugify(title)
    page_path = WIKI_DIR / category / f"{slug}.md"
    if page_path.exists():
        page_path.unlink()
        _log("delete", category, title)
        _update_index()
        return True
    return False


def list_pages(category: str | None = None) -> list[dict]:
    """List all wiki pages, optionally filtered by category."""
    results = []
    for cat_dir in sorted(WIKI_DIR.iterdir()):
        if not cat_dir.is_dir():
            continue
        cat = cat_dir.name
        if category and cat != category:
            continue
        for page in sorted(cat_dir.glob("*.md")):
            results.append({
                "slug": page.stem,
                "category": cat,
                "path": str(page),
            })
    return results


def search_wiki(query: str, max_results: int = 10) -> list[dict]:
    """Keyword search across all wiki pages."""
    q = query.lower()
    terms = q.split()
    results = []
    for cat_dir in WIKI_DIR.iterdir():
        if not cat_dir.is_dir():
            continue
        for page in cat_dir.glob("*.md"):
            text = page.read_text().lower()
            score = sum(1 for t in terms if t in text)
            if score > 0:
                # Extract a snippet around the first match
                snippet = ""
                for t in terms:
                    idx = text.find(t)
                    if idx >= 0:
                        start = max(0, idx - 60)
                        end = min(len(text), idx + 60)
                        snippet = text[start:end].replace("\n", " ")
                        break
                results.append({
                    "slug": page.stem,
                    "category": cat_dir.name,
                    "path": str(page),
                    "score": score,
                    "snippet": snippet,
                })
    results.sort(key=lambda x: x["score"], reverse=True)
    return results[:max_results]


def get_log(last_n: int = 20) -> list[str]:
    if not LOG_FILE.exists():
        return []
    lines = LOG_FILE.read_text().strip().split("\n")
    entries = [l for l in lines if l.startswith("- [")]
    return entries[-last_n:]


def _update_index():
    """Rebuild index.md from current wiki pages."""
    categories: dict[str, list[str]] = {}
    for cat_dir in sorted(WIKI_DIR.iterdir()):
        if not cat_dir.is_dir():
            continue
        cat = cat_dir.name
        pages = sorted(cat_dir.glob("*.md"))
        if pages:
            categories[cat] = [p.stem for p in pages]

    lines = ["# Context Graph Wiki", "", "## Pages by Category", ""]
    for cat, slugs in sorted(categories.items()):
        lines.append(f"### {cat.title()}")
        for s in slugs:
            lines.append(f"- [[{s}]]")
        lines.append("")

    INDEX_FILE.write_text("\n".join(lines) + "\n")


def lint_wiki() -> dict:
    """Check wiki health: orphan pages, broken wikilinks, isolated pages."""
    all_pages = {p["slug"] for p in list_pages()}
    all_links: set[str] = set()
    page_links: dict[str, set[str]] = {}

    for cat_dir in WIKI_DIR.iterdir():
        if not cat_dir.is_dir():
            continue
        for page in cat_dir.glob("*.md"):
            text = page.read_text()
            links = set(re.findall(r"\[\[([^\]]+)\]\]", text))
            slugified = {_slugify(l) for l in links}
            all_links.update(slugified)
            page_links[page.stem] = slugified

    broken = all_links - all_pages
    orphan = all_pages - all_links - {"index", "log"}
    isolated = [p for p, links in page_links.items() if not links and p not in ("index", "log")]

    return {
        "orphan_pages": sorted(orphan),
        "missing_pages": sorted(broken),
        "broken_wikilinks": sorted(broken),
        "isolated_pages": sorted(isolated),
        "total_pages": len(all_pages),
    }
