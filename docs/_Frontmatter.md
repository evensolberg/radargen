# Frontmatter

This repository uses the shared frontmatter standard in
`/Volumes/SSD/Source/_Common/Frontmatter.md`. Copy the template from there. It
defines every field, the allowed `type` and `status` values, the quoting rules,
and where revision history goes: a `## Revisions` table as the last section of
the document. There is no `revision_history` field in the frontmatter.

Change the standard in `_Common/Frontmatter.md`, not here. This file records only
what is specific to this repository.

## Specific to this repository

- **doc_id prefix:** `RG`, so IDs are `RG-NNN` (three digits, zero-padded).
- **Folders:** implementation plans live in `docs/plans/`, named `pr-NN-short-title.md`. Shipped plans move to `docs/plans/done/`, not the standard's `completed/`.
- **Older documents:** the PRD (`docs/product/radargen-prd.md`) and the architecture (`docs/design/architecture.md`) predate the standard. The PRD keeps its own frontmatter and version table. Convert them only when there's a reason to.
