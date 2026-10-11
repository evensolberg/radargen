---
tags:
  - plan
  - ordering
  - determinism
aliases: []
doc_id: RG-007
doc_name: pr-07-order
crumb_id: rg-efx
document_title: "PR 07 — `order.rs`: quadrant and item comparators: Implementation Plan"
created_date: 2026-10-11
synopsis: "The resolve stage: FR-10's quadrant order and its rank, FR-34's item order through total comparators, and FR-18's numbers, returned as a Plotted that layout and the HTML emitter read without reordering the Catalogue."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-11
reviewed_by:
  - rust-engineer
  - qa-expert
completed_date:
comments:
---
# PR 07 — `order.rs`: quadrant and item comparators: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-efx` · **Phase:** v1.0 · **Blocks:** `rg-48z` (PR 08)

**Goal:** Give every item its number and every quadrant its place. The same CSV numbers the same way on every run and every machine, whatever order its rows arrive in, and a stray space or a case difference never moves an item.

**Architecture:** `order.rs` is the pipeline's `resolve` stage (architecture §2): `Catalogue` in, `Plotted` out.
- `compare_quadrants` and `quadrant_order` give FR-10's order as a list of indices into `Catalogue::quadrants`, and `ranks` inverts it. The list is the one place the order is computed, so a later `--quadrants` (OQ-17) replaces the computation, not its consumers. The rank is FR-67's palette index.
- `compare_names` and `compare_items` are FR-34's chain: quadrant rank, ring in declared order, name key, trimmed name, CSV line. Every comparator is total, and the tests assert it (architecture §6).
- `resolve(&Catalogue) -> Plotted` sorts the plotted items, numbers them from 1 (FR-18), and gives FR-78's table order. It never reorders or mutates the `Catalogue`, whose indices stay first-appearance and CSV order.

**Tech Stack:** Rust 2024, `std` only. No new dependency.

**Spec:** `docs/product/radargen-prd.md` v0.46 — FR-9, FR-10, FR-18, FR-34, FR-35, FR-42, FR-67, FR-76, FR-78, NFR-4, D-7, D-29, D-33, D-53, D-60, D-63. Design: `docs/design/architecture.md` §2 (the `resolve` stage and the `Catalogue` side-channel), §3 (the `order.rs` row), §5 (the palette index is FR-10's rank), §6 (total comparators, pre-computed keys), §8 (OQ-9 (b) and OQ-17 live here).

## Global Constraints

- **Comparators never fold** (architecture §6). They read `Quadrant::key` and `Item::name_key`, which `model.rs` computed once, and FR-34's ordinal step reads `lib::trim(&item.name)`, which borrows and does not allocate. Never call `normalise`, `to_lowercase` or `str::trim` in `order.rs`.
- **Every string order is Unicode code point order**, which is `str::cmp` (D-53). Never `localeCompare`-style or UTF-16 ordering, and no locale.
- **The ordinal tiebreaks compare trimmed, unfolded values** (D-63): FR-34's name step is `trim(&a.name).cmp(trim(&b.name))`.
- **Every comparator is total.** FR-34 ends at `Item::line`, which no two parsed rows share; `quadrant_order` ends at the quadrant's index. Sorts are `sort_unstable_by`, so a comparator that returns `Equal` for distinct elements is a bug, not a stable-sort accident. Tests assert totality.
- **The `Catalogue` is read-only here.** `Item::quadrant` stays first-appearance order and is never a palette index (architecture §5); `Plotted` carries the orders as index lists.
- **No `HashMap` iteration** in output-affecting code (architecture §6). Everything here is `Vec` and explicit sorts.
- `cargo fmt` before every commit; CI runs `cargo fmt --check`.
- `cargo clippy --all-targets -- -D warnings` and `just lint` (pedantic + nursery + `unwrap_used`) are clean **at every commit**. No `#[allow]`/`#[expect]` without an inline reason.
- `just ci` passes before the PR opens. rustqual fails on a lower quality score than `rustqual-baseline.json`, so keep `resolve` an integration function (calls only) and the logic in the functions it calls (IOSP).
- Commits: `type(scope): description` (subject ≤ 72 characters, body lines ≤ 72), plus the session's `Co-Authored-By` trailer. Run `git-mit es` first. The commit hook rejects a body line over 72 characters, so check `git log -1` after committing.

## Review Focus

1. **Two names that differ only by a stray leading space.** ` Kafka` and `Kafka` must fall through to CSV order, not sort the spaced one first; and ` Zookeeper` must not jump ahead of `Airflow` (D-63, FR-76). *Test: Task 2, `names_compare_trimmed_so_a_space_cannot_move_a_name`.*
2. **The same export with its rows in a different order.** Every item keeps its number and every quadrant its place (D-7, NFR-4). *Tests: Task 1, `quadrant_order_does_not_depend_on_row_order`; Task 3, `numbering_does_not_depend_on_row_order`.*
3. **A non-ASCII name, including one above U+FFFF.** Order is by code point: `Zulu` before `Écosse`, and U+FF21 before U+1F600, which UTF-16 order would invert (D-53). *Tests: Task 1, `quadrants_order_by_code_point_above_ascii`; Task 2, `names_order_by_code_point_not_utf16`.*
4. **Unplotted rows between plotted ones, or a quadrant holding only unplotted rows.** Numbers stay contiguous over plotted items (FR-18), the quadrant still has its place (FR-42's empty legend group), and the table lists unplotted rows last in CSV order (FR-78). *Tests: Task 1, `a_quadrant_of_unplotted_rows_is_still_ordered`; Task 3, `numbers_are_contiguous_over_plotted_items`, `every_row_unplotted_numbers_nothing` and `table_rows_put_plotted_first_then_unplotted_in_csv_order`.*
5. **Names that start with a lowercase letter** — `gRPC`, `npm`, `k6`, `iOS`. They sort among the rest ignoring case, not after `Z` (FR-34). *Test: Task 2, `names_order_case_insensitively_before_ordinally`.*
6. **A ring set whose declared order is not alphabetical.** Within a quadrant, items follow the declared order, so the legend's numbers ascend down each group (FR-34, FR-35, D-33). *Test: Task 2, `items_order_by_quadrant_then_declared_ring_then_name`.*

## Crumbs this plan incorporates

| Crumb | What it asked | Where |
| --- | --- | --- |
| `rg-efx` | FR-10, FR-34, `compare_names`; every comparator total, asserted in tests | Tasks 1–2 |
| `rg-efx` (v0.37, D-53) | Fold with `str::to_lowercase`, order by code point; return the quadrant order as a list (OQ-17) | Tasks 1–2; the folding is `model.rs`'s, already done |
| `rg-efx` (PR 06 note) | Read `Item::name_key`, `Quadrant::key`, `Item::line`; return FR-10's order as indices plus a rank, without reordering the `Catalogue`; own FR-18's numbers, `None` for unplotted rows | Tasks 1–3 |
| `rg-efx` (v0.45, D-63) | FR-34's ordinal name tiebreak compares `lib::trim(name)`, not folded and not raw | Task 2 |
| `rg-48z` (PR 08) | Layout | Not here. It reads `Plotted` (Finishing step 3) |
| `rg-f27` (PR 14) | HTML output | Not here. It reads `Plotted::numbers` and `table_rows` for FR-78 and `accessible_name` (Finishing step 3) |

## Decisions taken in this plan (flag on review)

- **`Plotted` is index lists, not a copy.** `quadrants` is FR-10's order as indices into `Catalogue::quadrants`, `rank` its inverse, `items` the plotted items in FR-34's order as indices into `Catalogue::items`, and `numbers` each item's number aligned with `Catalogue::items`. Layout and the emitters read the `Catalogue` through these, so nothing is cloned and the `Catalogue`'s indices keep one meaning everywhere.
- **`numbers` is aligned with `Catalogue::items`, not keyed by `Item::line`.** Architecture §2 said "keyed by `Item::line`"; a `Vec<Option<usize>>` by item index is an O(1) look-up with no map, and every consumer already holds the index. Task 3 updates §2's sentence.
- **`compare_items` takes the rank slice**, because FR-34's first key is the quadrant's FR-10 position, which only exists once the quadrants are ordered. `compare_items(rank, a, b)` keeps it a plain function rather than a closure-capturing struct.
- **`compare_items` orders `ring` as `Option<usize>`.** `None` (unplotted) sorts before every ring, but `resolve` sorts only plotted items, so that never reaches a number; it only keeps the comparator total for the totality test, which includes unplotted items.
- **`compare_quadrants` breaks a key tie by label.** FR-9 groups quadrants by key, so keys are unique in a built `Catalogue` and the label never decides; `quadrant_order` then ends at the index. Both exist only so a hand-built `Catalogue` cannot make the sort unstable.
- **Numbers start at 1.** FR-34 says "numbered sequentially" and `accessible_name`'s format (`"3. Kafka, …"`) and the mockup start at 1.
- **`table_rows` lives here**, not in the HTML emitter. FR-78's order is plotted rows in FR-34's order, then unplotted rows in CSV order, and every ordering the output depends on is `order.rs`'s (architecture §3).
- **No version bump.** Nothing calls `resolve` until PR 14 wires the pipeline, so the binary is unchanged.

---

### Task 1: FR-10 — quadrant order and rank

**Files:**
- Create: `src/order.rs`
- Modify: `src/lib.rs` (add `pub mod order;`, alphabetically between `num` and `palette`)

**Interfaces:**
- Consumes: `model::{Catalogue, Quadrant}` (`Quadrant { key: String, label: String }`, `Catalogue::quadrants: Vec<Quadrant>`); in tests, `csv::parse::parse`, `model::build_catalogue`, `rings::parse_rings`, `diagnostics::Warnings`.
- Produces: `pub fn compare_quadrants(a: &Quadrant, b: &Quadrant) -> Ordering`; `pub fn quadrant_order(quadrants: &[Quadrant]) -> Vec<usize>`; the test helpers `catalogue`, `catalogue_with`, `csv`, `labels`.

Tasks 1 and 2 add only public functions. The private helpers (`ranks`, `item_order`, `numbers`) arrive in Task 3 with `resolve`, their only caller: `just lint` checks the library without its tests, so a private function only tests call fails it as dead code.

- [ ] **Step 1: Write the failing tests**

Create `src/order.rs`:

```rust
//! Every ordering the output depends on (architecture §2, §3): FR-10's
//! quadrant order, FR-34's item order, and FR-18's numbers taken from it.
//!
//! The comparators read the keys `model.rs` computed once and never fold
//! (FR-76, architecture §6). Every one is total, so the sorts are unstable
//! and no order depends on sort stability.

use std::cmp::Ordering;

use crate::model::Quadrant;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse::parse;
    use crate::diagnostics::Warnings;
    use crate::model::{Catalogue, build_catalogue};
    use crate::rings::parse_rings;

    /// A `name,quadrant,ring,description` CSV, one row per
    /// `(name, quadrant, ring)`. Data rows start on line 2.
    fn csv(rows: &[(&str, &str, &str)]) -> String {
        use std::fmt::Write as _;
        let mut text = String::from("name,quadrant,ring,description\n");
        for (name, quadrant, ring) in rows {
            writeln!(text, "{name},{quadrant},{ring},d").expect("writes to a String");
        }
        text
    }

    fn catalogue_with(csv: &str, rings: Option<&str>) -> Catalogue {
        let mut w = Warnings::default();
        let (table, errs) = parse(csv, &mut w).expect("parses");
        assert!(errs.is_empty(), "{errs:?}");
        let rings = parse_rings(rings).expect("valid rings");
        build_catalogue(&table, &rings, &mut w).expect("builds")
    }

    /// Under the default rings: Assess, Trial, Adopt, Retire.
    fn catalogue(csv: &str) -> Catalogue {
        catalogue_with(csv, None)
    }

    fn labels<'a>(cat: &'a Catalogue, order: &[usize]) -> Vec<&'a str> {
        order.iter().map(|&q| cat.quadrants[q].label.as_str()).collect()
    }

    /// FR-10: case-insensitive, so `beta` sits between `Alpha` and `Gamma`
    /// although byte order would put it last.
    #[test]
    fn quadrants_order_case_insensitively() {
        let cat = catalogue(&csv(&[
            ("A", "beta", "Adopt"),
            ("B", "Gamma", "Adopt"),
            ("C", "Alpha", "Adopt"),
        ]));
        let order = quadrant_order(&cat.quadrants);
        assert_eq!(labels(&cat, &order), ["Alpha", "beta", "Gamma"]);
        assert_eq!(order, [2, 0, 1]);
    }

    /// D-7: the order comes from the names alone, not from row order.
    #[test]
    fn quadrant_order_does_not_depend_on_row_order() {
        let a = catalogue(&csv(&[
            ("A", "Tool", "Adopt"),
            ("B", "Platform", "Adopt"),
            ("C", "Practice", "Adopt"),
        ]));
        let b = catalogue(&csv(&[
            ("C", "Practice", "Adopt"),
            ("A", "Tool", "Adopt"),
            ("B", "Platform", "Adopt"),
        ]));
        let expected = ["Platform", "Practice", "Tool"];
        assert_eq!(labels(&a, &quadrant_order(&a.quadrants)), expected);
        assert_eq!(labels(&b, &quadrant_order(&b.quadrants)), expected);
    }

    /// D-53: code point order, so `é` (U+00E9) sorts after `z` (U+007A).
    /// No locale puts `Écosse` before `Zulu` here.
    #[test]
    fn quadrants_order_by_code_point_above_ascii() {
        let cat = catalogue(&csv(&[("A", "Écosse", "Adopt"), ("B", "Zulu", "Adopt")]));
        assert_eq!(labels(&cat, &quadrant_order(&cat.quadrants)), ["Zulu", "Écosse"]);
    }

    /// FR-42: a quadrant whose rows are all unplotted keeps its place, so
    /// its legend group renders empty rather than vanishing.
    #[test]
    fn a_quadrant_of_unplotted_rows_is_still_ordered() {
        let cat = catalogue(&csv(&[("A", "Tool", "Adopt"), ("B", "Platform", "Rejected")]));
        assert_eq!(labels(&cat, &quadrant_order(&cat.quadrants)), ["Platform", "Tool"]);
    }

    /// Architecture §6: total even on a hand-built list with a repeated
    /// key, which FR-9's grouping never produces. The label breaks the
    /// key tie, then the index breaks an exact duplicate.
    #[test]
    fn the_quadrant_order_is_total() {
        let q = |key: &str, label: &str| Quadrant {
            key: key.to_string(),
            label: label.to_string(),
        };
        let quadrants = [q("tool", "Tool"), q("tool", "TOOL"), q("alpha", "Alpha"), q("tool", "Tool")];
        assert_eq!(quadrant_order(&quadrants), [2, 1, 0, 3]);
        assert_eq!(compare_quadrants(&quadrants[0], &quadrants[3]), Ordering::Equal);
        assert_eq!(compare_quadrants(&quadrants[1], &quadrants[0]), Ordering::Less);
    }
}
```

Add `pub mod order;` to `src/lib.rs` between `pub mod num;` and `pub mod palette;`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib order::`
Expected: FAIL to compile — `cannot find function quadrant_order` and `compare_quadrants`.

- [ ] **Step 3: Write the implementation**

In `src/order.rs`, between the `use` lines and `#[cfg(test)]`:

```rust
/// FR-10: case-insensitive alphabetical by FR-76's key, in code point
/// order (D-7, D-53).
///
/// FR-9 groups quadrants by key, so keys are unique in a built
/// `Catalogue` and the label never decides; it breaks a tie only in a
/// hand-built list.
#[must_use]
pub fn compare_quadrants(a: &Quadrant, b: &Quadrant) -> Ordering {
    a.key.cmp(&b.key).then_with(|| a.label.cmp(&b.label))
}

/// FR-10's order, as indices into `quadrants`.
///
/// This list is the one place the order is computed, so an explicit
/// `--quadrants` later (OQ-17) replaces this function, not its readers.
/// The index ends the comparison, so the order is total even for exact
/// duplicates.
#[must_use]
pub fn quadrant_order(quadrants: &[Quadrant]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..quadrants.len()).collect();
    order.sort_unstable_by(|&a, &b| {
        compare_quadrants(&quadrants[a], &quadrants[b]).then(a.cmp(&b))
    });
    order
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib order::`
Expected: 5 passed.

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && just lint`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git-mit es
git add src/order.rs src/lib.rs
git commit -m "feat(order): FR-10 quadrant order as an index list

Quadrants order case-insensitively by FR-76's key, in code point
order (D-7, D-53), returned as indices into Catalogue::quadrants so
a later --quadrants replaces one function (OQ-17).

Co-Authored-By: <the session's trailer>"
git log -1 --stat
```

---

### Task 2: FR-34 — the item comparators

**Files:**
- Modify: `src/order.rs`

**Interfaces:**
- Consumes: Task 1's `quadrant_order` and test helpers; `model::Item` (`line: usize`, `name: String`, `name_key: String`, `quadrant: usize`, `ring: Option<usize>`); `crate::trim`.
- Produces: `pub fn compare_names(a: &Item, b: &Item) -> Ordering`; `pub fn compare_items(rank: &[usize], a: &Item, b: &Item) -> Ordering`; the test helpers `rank_of` and `names_in_order`, which Task 3 rebuilds on `resolve`.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/order.rs`:

```rust
    /// Each quadrant's FR-10 position. Task 3 replaces this with
    /// `resolve(cat).rank`.
    fn rank_of(cat: &Catalogue) -> Vec<usize> {
        let mut rank = vec![0; cat.quadrants.len()];
        for (position, &q) in quadrant_order(&cat.quadrants).iter().enumerate() {
            rank[q] = position;
        }
        rank
    }

    /// The plotted items' names in FR-34's order. Task 3 replaces the body
    /// with `resolve(cat).items`.
    fn names_in_order(cat: &Catalogue) -> Vec<&str> {
        let rank = rank_of(cat);
        let mut plotted: Vec<&Item> = cat.items.iter().filter(|i| i.ring.is_some()).collect();
        plotted.sort_unstable_by(|a, b| compare_items(&rank, a, b));
        plotted.iter().map(|i| i.name.as_str()).collect()
    }

    /// FR-34, FR-35, D-33: quadrant (FR-10), then ring in the declared
    /// order — here not alphabetical — then name. An unplotted row has no
    /// number (FR-18), so it is not in the order. Three quadrants in
    /// first-appearance order Tool, Delivery, Platform give FR-10 order
    /// `[1, 2, 0]` and rank `[2, 0, 1]`, which differ, so passing the
    /// order where the rank belongs would fail here.
    #[test]
    fn items_order_by_quadrant_then_declared_ring_then_name() {
        let cat = catalogue_with(
            &csv(&[
                ("Zed", "Tool", "Hold"),
                ("Ops", "Delivery", "Adopt"),
                ("Beta", "Tool", "Adopt"),
                ("Alpha", "Tool", "Adopt"),
                ("Kube", "Platform", "Trial"),
                ("Old", "Platform", "Rejected"),
            ]),
            Some("Hold,Assess,Trial,Adopt"),
        );
        assert_eq!(names_in_order(&cat), ["Ops", "Kube", "Zed", "Alpha", "Beta"]);
    }

    /// FR-34's case-insensitive step decides before the ordinal one: real
    /// radars hold `gRPC`, `npm` and `k6`, and code point order alone
    /// would put every lowercase-initial name after `Z`.
    #[test]
    fn names_order_case_insensitively_before_ordinally() {
        let cat = catalogue(&csv(&[
            ("beta", "T", "Adopt"),
            ("Gamma", "T", "Adopt"),
            ("Alpha", "T", "Adopt"),
        ]));
        assert_eq!(names_in_order(&cat), ["Alpha", "beta", "Gamma"]);
    }

    /// FR-34: case-insensitively first, then ordinally, so names differing
    /// only in case do not fall through to row order.
    #[test]
    fn names_order_case_insensitively_then_by_case() {
        let cat = catalogue(&csv(&[
            ("kafka", "T", "Adopt"),
            ("Kafka", "T", "Adopt"),
            ("Airflow", "T", "Adopt"),
        ]));
        assert_eq!(names_in_order(&cat), ["Airflow", "Kafka", "kafka"]);
    }

    /// D-63: the ordinal step compares trimmed names. ` Kafka` and `Kafka`
    /// are then equal and fall to CSV line, where a raw comparison would
    /// put the spaced one first; and a leading space cannot move
    /// `Zookeeper` ahead of `Airflow`.
    #[test]
    fn names_compare_trimmed_so_a_space_cannot_move_a_name() {
        let cat = catalogue(&csv(&[("Kafka", "T", "Adopt"), (" Kafka", "T", "Adopt")]));
        assert_eq!(names_in_order(&cat), ["Kafka", " Kafka"]);
        let cat = catalogue(&csv(&[(" Zookeeper", "T", "Adopt"), ("Airflow", "T", "Adopt")]));
        assert_eq!(names_in_order(&cat), ["Airflow", " Zookeeper"]);
    }

    /// D-53: code point order, which is UTF-8 byte order. U+FF21 (FULLWIDTH
    /// A) sorts before U+1F600; UTF-16 code-unit order, the prototype's,
    /// would invert them, since U+1F600 starts with the surrogate U+D83D.
    #[test]
    fn names_order_by_code_point_not_utf16() {
        let cat = catalogue(&csv(&[("\u{1F600}", "T", "Adopt"), ("\u{FF21}", "T", "Adopt")]));
        assert_eq!(names_in_order(&cat), ["\u{FF21}", "\u{1F600}"]);
    }

    /// Architecture §6: FR-34's chain is total. Over items that collide on
    /// every key but the line — the same name in three spellings, across
    /// quadrants and rings, one unplotted — no two distinct items compare
    /// equal, and the order is antisymmetric and transitive.
    #[test]
    fn the_item_comparator_is_total() {
        let cat = catalogue(&csv(&[
            ("Kafka", "Tool", "Adopt"),
            (" Kafka", "Tool", "Adopt"),
            ("kafka", "Tool", "Adopt"),
            ("Kafka", "Tool", "Adopt"),
            ("Kafka", "tool", "Trial"),
            ("Kafka", "Platform", "Adopt"),
            ("Kafka", "Platform", "Rejected"),
            ("Airflow", "Platform", "Adopt"),
        ]));
        let rank = rank_of(&cat);
        let cmp = |a: usize, b: usize| compare_items(&rank, &cat.items[a], &cat.items[b]);
        let n = cat.items.len();
        for a in 0..n {
            for b in 0..n {
                assert_eq!(cmp(a, b) == Ordering::Equal, a == b, "{a} vs {b}");
                assert_eq!(cmp(a, b), cmp(b, a).reverse(), "{a} vs {b}");
                for c in 0..n {
                    if cmp(a, b).is_lt() && cmp(b, c).is_lt() {
                        assert!(cmp(a, c).is_lt(), "{a} < {b} < {c}");
                    }
                }
            }
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib order::`
Expected: FAIL to compile — `cannot find function compare_items` and `compare_names`.

- [ ] **Step 3: Write the implementation**

Change the imports at the top of `src/order.rs` to:

```rust
use std::cmp::Ordering;

use crate::model::{Item, Quadrant};
use crate::trim;
```

Add `Item` to the test module's imports: `use crate::model::{Catalogue, Item, build_catalogue};`.

Add after `quadrant_order`:

```rust
/// FR-34's name steps: FR-76's key, then the trimmed name ordinally, so
/// names differing only in case do not fall through to row order, and a
/// stray space cannot move a name (D-63).
///
/// D-53, and OQ-9 (b)'s home (architecture §8): both steps compare by
/// Unicode code point, which is `str::cmp`, UTF-8 byte order. That
/// differs from UTF-16 code-unit order, the prototype's `<`, for a
/// character above U+FFFF against one in U+E000–U+FFFF.
#[must_use]
pub fn compare_names(a: &Item, b: &Item) -> Ordering {
    a.name_key
        .cmp(&b.name_key)
        .then_with(|| trim(&a.name).cmp(trim(&b.name)))
}

/// FR-34: the quadrant's FR-10 position (`rank`, from [`quadrant_order`]),
/// then the ring in declared order, which is the legend's (FR-35, D-33),
/// then [`compare_names`], then CSV line, which no two rows share.
///
/// An unplotted item (`ring` is `None`) sorts before its quadrant's
/// plotted ones; [`resolve`] sorts only plotted items, so that only keeps
/// this total.
#[must_use]
pub fn compare_items(rank: &[usize], a: &Item, b: &Item) -> Ordering {
    rank[a.quadrant]
        .cmp(&rank[b.quadrant])
        .then_with(|| a.ring.cmp(&b.ring))
        .then_with(|| compare_names(a, b))
        .then_with(|| a.line.cmp(&b.line))
}
```

The `[`resolve`]` intra-doc link resolves only once Task 3 adds `resolve`. Until then write it as plain `` `resolve` `` and change it to a link in Task 3; `cargo doc` is not a gate, but a broken link is a rustdoc warning.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib order::`
Expected: 11 passed.

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && just lint`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git-mit es
git add src/order.rs
git commit -m "feat(order): FR-34 item comparators, total by CSV line

Items order by quadrant rank, declared ring, FR-76's name key, then
the trimmed name in code point order (D-53, D-63), ending at the CSV
line so no two rows compare equal. The tests assert totality, so the
sorts can be unstable.

Co-Authored-By: <the session's trailer>"
git log -1 --stat
```

---

### Task 3: `resolve` — `Plotted`, FR-18's numbers and FR-78's table order

**Files:**
- Modify: `src/order.rs`
- Create: `tests/order.rs`
- Modify: `docs/design/architecture.md` (§2 pipeline note, §3 `order.rs` row)

**Interfaces:**
- Consumes: Tasks 1–2's `quadrant_order` and `compare_items`; `model::Catalogue` (`items`, `quadrants`, `accessible_name(&Item, Option<usize>) -> String`).
- Produces, for PR 08 (layout) and PR 14 (HTML):

```rust
pub struct Plotted {
    pub quadrants: Vec<usize>,      // FR-10 order, indices into Catalogue::quadrants
    pub rank: Vec<usize>,           // rank[q]: quadrant q's FR-10 position; FR-67's palette index
    pub items: Vec<usize>,          // plotted items in FR-34 order; position i is number i + 1
    pub numbers: Vec<Option<usize>>, // aligned with Catalogue::items; None when unplotted (FR-18)
}
impl Plotted { pub fn table_rows(&self) -> Vec<usize> }
pub fn resolve(catalogue: &Catalogue) -> Plotted
```

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/order.rs`:

```rust
    /// FR-18: numbers run 1, 2, … over plotted items only, in FR-34's
    /// order, with no gap where an unplotted row sits.
    #[test]
    fn numbers_are_contiguous_over_plotted_items() {
        let cat = catalogue(&csv(&[
            ("B", "Tool", "Adopt"),
            ("Old", "Tool", "Rejected"),
            ("A", "Tool", "Adopt"),
        ]));
        let plotted = resolve(&cat);
        assert_eq!(plotted.items, [2, 0]);
        assert_eq!(plotted.numbers, [Some(2), None, Some(1)]);
    }

    /// FR-18, FR-42: with nothing plotted there are no numbers, and every
    /// quadrant is still ordered for its empty legend group.
    #[test]
    fn every_row_unplotted_numbers_nothing() {
        let cat = catalogue(&csv(&[("A", "Tool", "Gone"), ("B", "Platform", "Rejected")]));
        let plotted = resolve(&cat);
        assert_eq!(plotted.items, Vec::<usize>::new());
        assert_eq!(plotted.numbers, [None, None]);
        assert_eq!(labels(&cat, &plotted.quadrants), ["Platform", "Tool"]);
    }

    /// FR-78: plotted rows first in FR-34's order, then unplotted rows in
    /// CSV order, not interleaved.
    #[test]
    fn table_rows_put_plotted_first_then_unplotted_in_csv_order() {
        let cat = catalogue(&csv(&[
            ("Old2", "Tool", "Gone"),
            ("B", "Tool", "Adopt"),
            ("Old1", "Tool", "Rejected"),
            ("A", "Tool", "Adopt"),
        ]));
        assert_eq!(resolve(&cat).table_rows(), [3, 1, 0, 2]);
    }

    /// The rank inverts FR-10's order: `rank[q]` is quadrant `q`'s place,
    /// FR-67's palette index (architecture §5).
    #[test]
    fn rank_inverts_the_quadrant_order() {
        let cat = catalogue(&csv(&[
            ("A", "beta", "Adopt"),
            ("B", "Gamma", "Adopt"),
            ("C", "Alpha", "Adopt"),
        ]));
        let plotted = resolve(&cat);
        assert_eq!(plotted.quadrants, [2, 0, 1]);
        assert_eq!(plotted.rank, [1, 2, 0]);
    }

    /// An empty catalogue resolves to nothing, without panicking.
    #[test]
    fn an_empty_catalogue_resolves_to_nothing() {
        let plotted = resolve(&catalogue("name,quadrant,ring,description\n"));
        assert_eq!(plotted, Plotted::default());
    }
```

Create `tests/order.rs`:

```rust
//! CSV in, numbers out: `parse`, `build_catalogue` and `resolve` on whole
//! files (architecture §2).

use radargen::csv::parse::parse;
use radargen::diagnostics::Warnings;
use radargen::model::{Catalogue, build_catalogue};
use radargen::order::{Plotted, resolve};
use radargen::rings::parse_rings;

const EXAMPLE: &str = include_str!("fixtures/three-quadrant.csv");

fn resolved(csv: &str) -> (Catalogue, Plotted) {
    let mut w = Warnings::default();
    let (table, errs) = parse(csv, &mut w).expect("parses");
    assert!(errs.is_empty(), "{errs:?}");
    let rings = parse_rings(None).expect("default");
    let cat = build_catalogue(&table, &rings, &mut w).expect("builds");
    let plotted = resolve(&cat);
    (cat, plotted)
}

/// Each item's name and number, in CSV order.
fn numbered(cat: &Catalogue, plotted: &Plotted) -> Vec<(String, Option<usize>)> {
    cat.items
        .iter()
        .zip(&plotted.numbers)
        .map(|(item, &n)| (item.name.clone(), n))
        .collect()
}

/// PRD §6.1.6's example (SC-2) under the default rings, Assess, Trial,
/// Adopt, Retire: Platform, Practice, Tool (FR-10), and within Tool,
/// Trial before Adopt before Retire (FR-34, D-33).
#[test]
fn the_prd_example_numbers_in_legend_order() {
    let (cat, plotted) = resolved(EXAMPLE);
    let quadrants: Vec<_> = plotted
        .quadrants
        .iter()
        .map(|&q| cat.quadrants[q].label.as_str())
        .collect();
    assert_eq!(quadrants, ["Platform", "Practice", "Tool"]);
    assert_eq!(plotted.rank, [2, 0, 1]);
    assert_eq!(
        plotted.numbers,
        [Some(5), Some(4), Some(3), Some(6), Some(1), Some(2), None]
    );

    // FR-50's names read FR-34's numbers; the unplotted row has none.
    let kubernetes = &cat.items[4];
    assert_eq!(
        cat.accessible_name(kubernetes, plotted.numbers[4]),
        "1. Kubernetes, Default; Platform quadrant, Adopt ring"
    );
    let legacy = &cat.items[6];
    assert_eq!(
        cat.accessible_name(legacy, plotted.numbers[6]),
        "Legacy Ticketing-Driven Change; Practice quadrant, Rejected ring; not on the radar"
    );
}

/// NFR-4, D-7: the same rows in reverse order give every item the same
/// number and every quadrant the same place.
#[test]
fn numbering_does_not_depend_on_row_order() {
    let mut lines: Vec<&str> = EXAMPLE.lines().collect();
    let header = lines.remove(0);
    lines.reverse();
    let reversed = format!("{header}\n{}\n", lines.join("\n"));

    let (cat, plotted) = resolved(EXAMPLE);
    let (rcat, rplotted) = resolved(&reversed);
    let mut forward = numbered(&cat, &plotted);
    let mut backward = numbered(&rcat, &rplotted);
    forward.sort();
    backward.sort();
    assert_eq!(forward, backward);

    let labels = |c: &Catalogue, p: &Plotted| -> Vec<String> {
        p.quadrants.iter().map(|&q| c.quadrants[q].label.clone()).collect()
    };
    assert_eq!(labels(&cat, &plotted), labels(&rcat, &rplotted));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib order:: && cargo test --test order`
Expected: FAIL to compile — `cannot find function resolve`, `cannot find type Plotted`.

`an_empty_catalogue_resolves_to_nothing` assumes `build_catalogue` accepts a header with no data rows. If it returns an error instead, build the empty `Catalogue` by hand (`Catalogue { columns: vec![], quadrants: vec![], rings: vec![], items: vec![] }`) and say so in the report; do not change `model.rs`.

- [ ] **Step 3: Write the implementation**

Change the model import to `use crate::model::{Catalogue, Item, Quadrant};`. In the test module, `Catalogue` and `Item` now come through `super::*`, so its model import becomes `use crate::model::build_catalogue;`.

Rebuild Task 2's test helpers on `resolve`, so the tests exercise the code that ships rather than a copy of it:

```rust
    fn rank_of(cat: &Catalogue) -> Vec<usize> {
        resolve(cat).rank
    }

    fn names_in_order(cat: &Catalogue) -> Vec<&str> {
        resolve(cat)
            .items
            .iter()
            .map(|&i| cat.items[i].name.as_str())
            .collect()
    }
```

Add to `src/order.rs`, before `compare_quadrants`:

```rust
/// The `resolve` stage's output (architecture §2): every order the
/// output depends on, as indices into the [`Catalogue`], which is never
/// reordered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plotted {
    /// FR-10's order, as indices into [`Catalogue::quadrants`]. Layout,
    /// the legend and the palette read this list (OQ-17).
    pub quadrants: Vec<usize>,
    /// `rank[q]` is quadrant `q`'s position in [`Plotted::quadrants`]:
    /// FR-67's palette index, never [`Item::quadrant`] (architecture §5).
    pub rank: Vec<usize>,
    /// The plotted items in FR-34's order, as indices into
    /// [`Catalogue::items`]. The item at position `i` is number `i + 1`.
    pub items: Vec<usize>,
    /// FR-18: each item's number, aligned with [`Catalogue::items`];
    /// `None` for an unplotted item. FR-78's number column and FR-50's
    /// accessible names read it.
    pub numbers: Vec<Option<usize>>,
}

impl Plotted {
    /// FR-78's row order: plotted items in FR-34's order, then unplotted
    /// ones in CSV order, as indices into [`Catalogue::items`].
    #[must_use]
    pub fn table_rows(&self) -> Vec<usize> {
        let unplotted = self
            .numbers
            .iter()
            .enumerate()
            .filter(|(_, number)| number.is_none())
            .map(|(i, _)| i);
        self.items.iter().copied().chain(unplotted).collect()
    }
}

/// FR-10, FR-18, FR-34: order the quadrants and the plotted items, and
/// number the items from 1.
#[must_use]
pub fn resolve(catalogue: &Catalogue) -> Plotted {
    let quadrants = quadrant_order(&catalogue.quadrants);
    let rank = ranks(&quadrants);
    let items = item_order(catalogue, &rank);
    let numbers = numbers(&items, catalogue.items.len());
    Plotted {
        quadrants,
        rank,
        items,
        numbers,
    }
}
```

Add after `compare_items`:

```rust
/// The inverse of `order`: `rank[q]` is quadrant `q`'s position in it,
/// which is FR-67's palette index (architecture §5).
fn ranks(order: &[usize]) -> Vec<usize> {
    let mut rank = vec![0; order.len()];
    for (position, &q) in order.iter().enumerate() {
        rank[q] = position;
    }
    rank
}

/// The plotted items in FR-34's order, as indices into
/// `catalogue.items`. Unplotted items get no number (FR-18), so they are
/// not here.
fn item_order(catalogue: &Catalogue, rank: &[usize]) -> Vec<usize> {
    let mut order: Vec<usize> = catalogue
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.ring.is_some())
        .map(|(i, _)| i)
        .collect();
    order.sort_unstable_by(|&a, &b| {
        compare_items(rank, &catalogue.items[a], &catalogue.items[b])
    });
    order
}

/// FR-18: number `order`'s items from 1, in a list of `len` aligned with
/// `Catalogue::items`; every other item is `None`.
fn numbers(order: &[usize], len: usize) -> Vec<Option<usize>> {
    let mut numbers = vec![None; len];
    for (position, &i) in order.iter().enumerate() {
        numbers[i] = Some(position + 1);
    }
    numbers
}
```

In `compare_items`'s doc comment, change `` `resolve` `` back to the link [`resolve`] if Task 2 wrote it plain.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib order:: && cargo test --test order`
Expected: 16 passed in the library, 2 in `tests/order.rs`.

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && just lint`
Expected: clean.

- [ ] **Step 5: Update the architecture**

In `docs/design/architecture.md`:

1. §2, the sentence ending "The HTML emitter also reads FR-18's numbers from `resolve`'s output, keyed by `Item::line`, for FR-78's number column and FR-50's accessible names." Replace the whole sentence with: "The HTML emitter also reads `Plotted::numbers` (aligned with `Catalogue::items`) for FR-78's number column and FR-50's accessible names, and `Plotted::table_rows` for FR-78's row order."
2. §2, after the paragraph beginning "**`Catalogue` therefore keeps the CSV's shape**", add:

```markdown
**`Plotted` is index lists into the `Catalogue`, not a copy of it.** `resolve`
returns FR-10's quadrant order and its inverse rank (FR-67's palette index),
the plotted items in FR-34's order, and FR-18's numbers aligned with
`Catalogue::items`. The `Catalogue`'s own indices stay first-appearance and CSV
order, so `Item::quadrant` means one thing everywhere.
```

3. §3, the `src/order.rs` row: replace the description cell with "`resolve` → `Plotted`: FR-10's quadrant order and rank (`compare_quadrants`, `quadrant_order`), FR-34's item order (`compare_names`, `compare_items`), FR-18's numbers and FR-78's table order", and the status cell with "*exists*".

- [ ] **Step 6: Run the gate and commit**

Run: `just ci`
Expected: pass. If rustqual reports a quality drop, look first at `resolve` (it must only call functions) and `table_rows`.

```bash
git-mit es
git add src/order.rs tests/order.rs docs/design/architecture.md
git commit -m "feat(order): resolve to Plotted with FR-18's numbers

resolve orders the quadrants and plotted items and numbers them from
1, returning index lists into the Catalogue rather than reordering
it. table_rows gives FR-78's order: plotted first, then unplotted in
CSV order. Architecture §2 and §3 describe Plotted.

Co-Authored-By: <the session's trailer>"
git log -1 --stat
```

If rustqual's score rose, run `just quality-baseline` and include `rustqual-baseline.json` in this commit.

---

## Finishing

1. **Reviews.** Code reviewer over the whole branch, then the local multi-agent review in Copilot's seat (out of credits until November 2026; say so in the PR body), then simplify, then a final code review. Iterate each until clean.
2. **Close the crumb:** `crumbs close rg-efx`.
3. **Hand-off notes:**
   - `rg-48z` (PR 08): layout reads `Plotted::quadrants` for sector order and `Plotted::items` for marker order; a quadrant's series entry is `Plotted::rank[q]` (FR-67), never `Item::quadrant`.
   - `rg-f27` (PR 14): the HTML emitter reads `Plotted::numbers` for FR-78's number column and `accessible_name`, and `Plotted::table_rows()` for FR-78's row order.
4. **Move this plan** to `docs/plans/done/pr-07-order.md` and set `status: done` and `completed_date`.
5. **No version bump, no tag:** nothing calls `resolve` until PR 14, so the binary is unchanged.
6. Commit, push, open the PR with no reviewer flag, and stop for the maintainer's merge.
