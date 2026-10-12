---
id: rg-o70
title: Config carries the input path
status: open
type: task
priority: 4
tags:
- pipeline
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# Config carries the input path

From the PR 14a final review. pipeline::steps reads cli.csv_path while every other stage reads Config (architecture 7: the resolved run). Move the input path into Config, which PR 19 needs anyway when the CSV path resolves against a config file.
