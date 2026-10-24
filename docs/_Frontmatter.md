# Frontmatter

Documents under `docs/` start with this frontmatter block. It is the maintainer's shared standard across their repositories, copied here so the repository stands on its own.

```yaml
---
tags: []
aliases: []
doc_id:
doc_name:
crumb_id:
document_title:
created_date:
synopsis:
status:
type:
revision:
review_date:
reviewed_by: []
completed_date:
comments:
---
```

## Fields

- `tags`, `aliases` and `reviewed_by` are YAML lists.
- `doc_id` is `RG-NNN` (three digits, zero-padded), unique in the repository. Cite a document by its `doc_id` rather than its path, since the ID survives a move.
- `doc_name` is the file name in kebab case, without its extension.
- `crumb_id` links the document to the crumb that produced it, where there is one.
- `document_title` matches the H1 heading.
- `created_date`, `review_date` and `completed_date` are `YYYY-MM-DD`.
- `type` is one of `Reference`, `Guide`, `ADR`, `Plan`, `Spec` or `Note`. A Spec says what to build and why; a Plan says how and in what order. The first tag is the lowercase form of `type`.
- `status` is one of `draft`, `active`, `complete`, `superseded` or `archived`. An ADR maps its own vocabulary: proposed is `draft`, accepted is `active`, deprecated or superseded is `superseded`, rejected is `archived`.
- `revision` is the current revision, such as `"1.0"`, and matches the last row of the Revisions table.

## Revision history

History is not kept in the frontmatter, where a long list breaks Obsidian's properties panel and eventually the YAML parse. It goes in a `## Revisions` section, the last in the document:

```markdown
## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | YYYY-MM-DD | Initial version |
```

Bump `revision` and add a row together.

## Quoting

`synopsis` and `comments` are prose in a double-quoted YAML string. A double quote inside one ends it early and breaks the whole block, so use single quotes inside, or a block scalar (`comments: >-` with the text indented beneath). Check a block parses before committing: `yq --front-matter=extract '.' <file>` exits non-zero when it does not.

## Specific to this repository

- **doc_id prefix:** `RG`.
- **Folders:** implementation plans live in `docs/plans/`, named `pr-NN-short-title.md`; shipped plans move to `docs/plans/done/`.
- **Older documents:** the PRD (`docs/product/radargen-prd.md`) and the architecture (`docs/design/architecture.md`) predate the standard. The PRD keeps its own frontmatter and version table.
