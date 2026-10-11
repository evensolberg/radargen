---
id: rg-o6r
title: Type Fatal's column as ColumnKind, not &'static str
status: open
type: task
priority: 4
tags:
- model
- diagnostics
created: 2026-10-10
updated: 2026-10-11
blocks:
- rg-f27
phase: ''
---

# Type Fatal's column as ColumnKind, not &'static str

From the PR 06 plan review (RG-006 rev 1.3, architecture). After PR 06 the canonical column names are spelled in ColumnKind::of, ColumnKind::as_str, DateColumn::as_str, RowKind::column_name and the MissingColumn/NoHeader messages. Fatal::{MissingColumn, EmptyRequired} hold &'static str only because ColumnKind lives in model.rs, which depends on diagnostics.rs. Move ColumnKind (of, as_str) into diagnostics.rs beside DateColumn or a leaf module, type the variants' column as ColumnKind, and have RowKind::column_name use ColumnKind::X.as_str().
