# Wiki Lint Workflow

Run `wiki_store.lint_wiki()` periodically to check health.

## Issues Detected

- **orphan_pages**: Pages not linked from any other page. Add wikilinks
  or delete if stale.
- **missing_pages**: Wikilinks pointing to pages that don't exist. Either
  create the page or fix the link.
- **broken_wikilinks**: Same as missing_pages.
- **isolated_pages**: Pages with no outgoing wikilinks. Add cross-references.

## Fix Workflow

1. Run lint.
2. For each missing page: create it if the entity exists in the graph,
   otherwise fix the broken link.
3. For each orphan page: add a wikilink from the relevant summary or
   topic page.
4. For each isolated page: add at least one outgoing wikilink.
