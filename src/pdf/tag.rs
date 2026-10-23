//! The structure a tagged PDF carries (FR-114): one tree of standard roles
//! for the whole document, built while the pages are drawn, and the marked
//! content that ties each drawn leaf to it.
//!
//! Layout says what each group is ([`Semantics`]); this says it in PDF. A
//! keyed element (the table, a list, a row, a row's cell) resumes when a
//! later page opens it again, so a split row is one `TR` and a ring broken
//! across pages one `L`. Marks go on leaves only: inside the radar's
//! figure every text and shape is the figure's, halos aside; elsewhere a
//! text is its nearest element's, or a paragraph's of its own, and a shape
//! is an artifact. `Decorative` and `Pagination` groups are artifacts whole
//! and opaque: nothing inside one opens an element or another artifact.

use std::collections::BTreeMap;
use std::num::{NonZeroU8, NonZeroUsize};

use pdf_writer::types::{ArtifactType, StructRole};
use pdf_writer::{Content, Name, TextStr};

use super::integer;
use super::leaf::Leaf;
use crate::diagnostics::Fatal;
use crate::svg::{ListKey, Scope, Semantics, Structure};

/// A standard structure role (ISO 32000-1, 14.8.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The root.
    Document,
    /// A heading of this level, 1 to 6.
    Heading(NonZeroU8),
    /// A paragraph.
    P,
    /// The radar.
    Figure,
    /// A list.
    L,
    /// A list item.
    Li,
    /// A list item's label.
    Lbl,
    /// A list item's body.
    LBody,
    /// The table.
    Table,
    /// A table row.
    Tr,
    /// A header cell, and what it heads.
    Th(Scope),
    /// A data cell, and the columns it spans when several.
    Td(Option<NonZeroUsize>),
}

impl Role {
    /// The standard role: the element's `/S`, and the tag of the marked
    /// content under it, so the two cannot disagree.
    #[must_use]
    pub const fn struct_role(self) -> StructRole {
        match self {
            Self::Document => StructRole::Document,
            Self::Heading(level) => match level.get() {
                1 => StructRole::H1,
                2 => StructRole::H2,
                3 => StructRole::H3,
                4 => StructRole::H4,
                5 => StructRole::H5,
                _ => StructRole::H6,
            },
            Self::P => StructRole::P,
            Self::Figure => StructRole::Figure,
            Self::L => StructRole::L,
            Self::Li => StructRole::LI,
            Self::Lbl => StructRole::Lbl,
            Self::LBody => StructRole::LBody,
            Self::Table => StructRole::Table,
            Self::Tr => StructRole::TR,
            Self::Th(_) => StructRole::TH,
            Self::Td(_) => StructRole::TD,
        }
    }
}

/// A child of an element: another element, or a marked sequence on a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kid {
    /// An element, by index.
    Element(usize),
    /// A marked-content sequence: its page's index and its MCID.
    Mcid {
        /// The page's index.
        page: usize,
        /// The sequence's MCID on that page.
        mcid: i32,
    },
}

/// One structure element.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Its role.
    pub role: Role,
    /// Its parent's index; `None` for the root.
    pub parent: Option<usize>,
    /// Its alternate description: the radar's name (D-75).
    pub alt: Option<String>,
    /// Its children, in reading order.
    pub kids: Vec<Kid>,
    /// Its Layout `BBox` in page space, left, bottom, right, top: the
    /// figure's, and a table's held on one page (rg-2qs).
    pub bbox: Option<[f32; 4]>,
}

/// The finished tree: element 0 is the document; each page's parent-tree
/// array maps its MCIDs, in order, to their elements.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    /// Every element, the root first.
    pub elements: Vec<Element>,
    /// Per page, the element of each MCID in order.
    pub parents: Vec<Vec<usize>>,
}

/// What an element resumes by on a later page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Table,
    HeaderRow,
    List(ListKey),
    Row(usize),
    Cell(usize, usize),
}

/// An open element on the current page: its index, and for a row the next
/// cell's position.
#[derive(Debug, Clone, Copy)]
struct Frame {
    element: usize,
    row: Option<usize>,
    cells: usize,
}

/// What [`Tagger::open`] opened, for [`Tagger::close`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Opened {
    /// Nothing: a group with no structure of its own.
    Nothing,
    /// An element, now on the stack.
    Element,
    /// The radar's figure, now on the stack: inside it nothing else opens.
    Figure,
    /// An artifact, now open in the content stream.
    Artifact,
}

/// Whether a leaf was marked, for [`Tagger::end`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Marked(bool);

/// The tree being built, and where the walk is in it.
#[derive(Debug)]
pub struct Tagger {
    elements: Vec<Element>,
    keys: BTreeMap<Key, usize>,
    stack: Vec<Frame>,
    /// How deep the walk is inside artifacts; inside one, nothing is marked.
    artifacts: usize,
    /// How deep inside the radar's figure.
    figures: usize,
    parents: Vec<Vec<usize>>,
    /// The page each boxed figure or table was first given its box on: a
    /// second page drops the box, since one box cannot hold two pages.
    boxed_on: BTreeMap<usize, usize>,
    /// The first count past what the PDF can number (rg-969), which
    /// [`into_tree`](Self::into_tree) returns: the file is not written.
    too_large: Option<Fatal>,
}

impl Default for Tagger {
    fn default() -> Self {
        Self::new()
    }
}

impl Tagger {
    /// A tree holding the document alone.
    #[must_use]
    pub fn new() -> Self {
        Self {
            elements: vec![Element {
                role: Role::Document,
                parent: None,
                alt: None,
                kids: Vec::new(),
                bbox: None,
            }],
            keys: BTreeMap::new(),
            stack: Vec::new(),
            artifacts: 0,
            figures: 0,
            parents: Vec::new(),
            boxed_on: BTreeMap::new(),
            too_large: None,
        }
    }

    /// Starts the next page: its MCIDs count from 0, under the document.
    pub fn begin_page(&mut self) {
        self.parents.push(Vec::new());
        self.stack = vec![Frame {
            element: 0,
            row: None,
            cells: 0,
        }];
        self.artifacts = 0;
        self.figures = 0;
    }

    /// The tree, once every page is drawn, without the elements whose
    /// content drew nothing (rg-969): a legend `LI` or `L` with no text
    /// would be an empty node to assistive technology. A `TD` or `TH`
    /// stays however empty, so every row keeps its cell count.
    ///
    /// # Errors
    ///
    /// [`Fatal::PdfTooLarge`] when a page held more marked sequences than
    /// a PDF can number.
    pub fn into_tree(self) -> Result<Tree, Fatal> {
        if let Some(error) = self.too_large {
            return Err(error);
        }
        // A child always follows its parent, so walking back settles each
        // element's children before it.
        let mut live = vec![false; self.elements.len()];
        for (i, element) in self.elements.iter().enumerate().rev() {
            let kept = i == 0
                || matches!(element.role, Role::Td(_) | Role::Th(_))
                || element.kids.iter().any(|kid| match kid {
                    Kid::Mcid { .. } => true,
                    Kid::Element(e) => {
                        // Else its slot is not settled yet, and a live
                        // child would be pruned.
                        debug_assert!(*e > i, "element {e} is a kid of {i}, which follows it");
                        live.get(*e).copied().unwrap_or(false)
                    }
                });
            if let Some(slot) = live.get_mut(i) {
                *slot = kept;
            }
        }
        // Each kept element's new index, in the old order.
        let mut count = 0;
        let renumbered: Vec<Option<usize>> = live
            .iter()
            .map(|&kept| {
                kept.then(|| {
                    count += 1;
                    count - 1
                })
            })
            .collect();
        let new = |old: usize| renumbered.get(old).copied().flatten();
        let elements = self
            .elements
            .into_iter()
            .zip(&live)
            .filter(|(_, kept)| **kept)
            .map(|(element, _)| Element {
                parent: element.parent.and_then(new),
                kids: element
                    .kids
                    .iter()
                    .filter_map(|kid| match *kid {
                        Kid::Element(e) => new(e).map(Kid::Element),
                        mcid @ Kid::Mcid { .. } => Some(mcid),
                    })
                    .collect(),
                ..element
            })
            .collect();
        // An element holding an MCID is always kept: dropping one here
        // would move every later MCID on its page to the wrong element.
        let parents = self
            .parents
            .iter()
            .map(|page| {
                page.iter()
                    .filter_map(|&e| {
                        let kept = new(e);
                        debug_assert!(kept.is_some(), "element {e} holds an MCID but was pruned");
                        kept
                    })
                    .collect()
            })
            .collect();
        Ok(Tree { elements, parents })
    }

    /// Opens what `semantics` means, writing an artifact's mark to `out`.
    pub fn open(&mut self, out: &mut Content, semantics: &Semantics) -> Opened {
        if self.artifacts > 0 {
            return Opened::Nothing;
        }
        match semantics {
            Semantics::Decorative => self.artifact(out, ArtifactType::Layout),
            Semantics::Pagination => self.artifact(out, ArtifactType::Pagination),
            _ if self.figures > 0 => Opened::Nothing,
            Semantics::Heading(level) => self.element(Role::Heading(*level), None, None),
            Semantics::Group(name) => {
                self.figures += 1;
                let _ = self.element(Role::Figure, None, Some(name.clone()));
                Opened::Figure
            }
            Semantics::LegendRow(_) => self.element(Role::Li, None, None),
            Semantics::Structure(structure) => self.structure(*structure),
            Semantics::None
            | Semantics::Marker(_)
            | Semantics::Indicator(_)
            | Semantics::FocusRing(_)
            | Semantics::Selection(_) => Opened::Nothing,
        }
    }

    /// Closes what [`open`](Self::open) opened.
    pub fn close(&mut self, out: &mut Content, opened: Opened) {
        match opened {
            Opened::Nothing => {}
            Opened::Element => {
                self.stack.pop();
            }
            Opened::Figure => {
                self.stack.pop();
                self.figures = self.figures.saturating_sub(1);
            }
            Opened::Artifact => {
                self.artifacts = self.artifacts.saturating_sub(1);
                out.end_marked_content();
            }
        }
    }

    /// Gives the figure or table that `opened` just opened its box on this
    /// page, in page space (rg-2qs); nothing else takes one, so a box on a
    /// group that opened nothing never lands on an ancestor. The box is the
    /// element's own, so pruning carries it. A table given one on a second
    /// page keeps none.
    pub fn extent(&mut self, opened: Opened, bbox: [f32; 4]) {
        if !matches!(opened, Opened::Element | Opened::Figure) {
            return;
        }
        let at = self.top();
        let page = self.parents.len().saturating_sub(1);
        let Some(element) = self.elements.get_mut(at) else {
            return;
        };
        if !matches!(element.role, Role::Figure | Role::Table) {
            return;
        }
        let first = *self.boxed_on.entry(at).or_insert(page);
        element.bbox = (first == page).then_some(bbox);
    }

    /// Marks `leaf` as FR-114 says: inside an artifact, nothing more; the
    /// ground and a halo as artifacts; a text as an MCID under its nearest
    /// element, or a paragraph of its own; a shape as the figure's inside
    /// the radar, else an artifact.
    pub fn leaf(&mut self, out: &mut Content, leaf: &Leaf) -> Marked {
        match leaf {
            Leaf::Ground | Leaf::Halo => self.ornament(out),
            Leaf::Text { actual } => self.text(out, actual.as_deref()),
            Leaf::Shape => self.shape(out),
        }
    }

    /// A text leaf: an MCID under its nearest element, or under a
    /// paragraph of its own; `actual_text` when its words are not its
    /// drawn lines (D-88). Inside an artifact it is not marked again.
    fn text(&mut self, out: &mut Content, actual_text: Option<&str>) -> Marked {
        if self.artifacts > 0 {
            return Marked(false);
        }
        let top = self.top();
        let element = if top == 0 {
            // Text no element claims is a paragraph of its own.
            let p = self.push_element(Role::P, 0, None);
            self.adopt(0, Kid::Element(p));
            p
        } else {
            top
        };
        self.mcid(out, element, actual_text);
        Marked(true)
    }

    /// A shape leaf: the figure's inside the radar, else an artifact.
    fn shape(&mut self, out: &mut Content) -> Marked {
        if self.artifacts > 0 {
            return Marked(false);
        }
        if self.figures > 0 {
            let top = self.top();
            self.mcid(out, top, None);
        } else {
            out.begin_marked_content(Name(b"Artifact"));
        }
        Marked(true)
    }

    /// A leaf that is always an artifact: the ground, a halo.
    fn ornament(&self, out: &mut Content) -> Marked {
        if self.artifacts > 0 {
            return Marked(false);
        }
        out.begin_marked_content(Name(b"Artifact"));
        Marked(true)
    }

    /// Ends a marked leaf.
    pub fn end(out: &mut Content, marked: Marked) {
        if marked.0 {
            out.end_marked_content();
        }
    }

    fn structure(&mut self, structure: Structure) -> Opened {
        match structure {
            Structure::List(key) => self.keyed(Key::List(key), Role::L),
            Structure::ListItem => self.element(Role::Li, None, None),
            Structure::Label => self.element(Role::Lbl, None, None),
            Structure::Body => self.element(Role::LBody, None, None),
            Structure::Table => self.keyed(Key::Table, Role::Table),
            Structure::HeaderRow => self.keyed(Key::HeaderRow, Role::Tr),
            Structure::Row(row) => {
                let opened = self.keyed(Key::Row(row), Role::Tr);
                if let Some(frame) = self.stack.last_mut() {
                    frame.row = Some(row);
                }
                opened
            }
            Structure::Header(scope) => self.cell(Role::Th(scope)),
            Structure::Cell { span, .. } => self.cell(Role::Td(span)),
            Structure::Paragraph => self.element(Role::P, None, None),
        }
    }

    /// A cell of the open row, keyed by its row and its position in it, so
    /// a split row's later part resumes its cells.
    fn cell(&mut self, role: Role) -> Opened {
        let Some(frame) = self.stack.last_mut() else {
            return Opened::Nothing;
        };
        let position = frame.cells;
        frame.cells += 1;
        match frame.row {
            Some(row) => self.keyed(Key::Cell(row, position), role),
            None => self.element(role, None, None),
        }
    }

    fn keyed(&mut self, key: Key, role: Role) -> Opened {
        self.element(role, Some(key), None)
    }

    /// Opens an element under the current one, or resumes the one `key`
    /// names.
    fn element(&mut self, role: Role, key: Option<Key>, alt: Option<String>) -> Opened {
        let parent = self.top();
        let resumed = key.and_then(|key| self.keys.get(&key).copied());
        let element = resumed.unwrap_or_else(|| {
            let element = self.push_element(role, parent, alt);
            self.adopt(parent, Kid::Element(element));
            if let Some(key) = key {
                self.keys.insert(key, element);
            }
            element
        });
        self.stack.push(Frame {
            element,
            row: None,
            cells: 0,
        });
        Opened::Element
    }

    fn artifact(&mut self, out: &mut Content, kind: ArtifactType) -> Opened {
        self.artifacts += 1;
        out.begin_marked_content_with_properties(Name(b"Artifact"))
            .properties()
            .artifact()
            .kind(kind);
        Opened::Artifact
    }

    fn top(&self) -> usize {
        self.stack.last().map_or(0, |frame| frame.element)
    }

    fn push_element(&mut self, role: Role, parent: usize, alt: Option<String>) -> usize {
        self.elements.push(Element {
            role,
            parent: Some(parent),
            alt,
            kids: Vec::new(),
            bbox: None,
        });
        self.elements.len() - 1
    }

    fn adopt(&mut self, parent: usize, kid: Kid) {
        if let Some(element) = self.elements.get_mut(parent) {
            element.kids.push(kid);
        }
    }

    /// The next MCID on this page, under `element`.
    fn mcid(&mut self, out: &mut Content, element: usize, actual_text: Option<&str>) {
        let page = self.parents.len().saturating_sub(1);
        let Some(parents) = self.parents.last_mut() else {
            return;
        };
        let mcid = match integer(parents.len(), "marked sequences on one page") {
            Ok(mcid) => mcid,
            Err(error) => {
                // The page is discarded with the error, so its stream
                // need not be balanced.
                self.too_large.get_or_insert(error);
                return;
            }
        };
        parents.push(element);
        let role = self.elements.get(element).map_or(Role::P, |e| e.role);
        let mut marked = out.begin_marked_content_with_properties(role.struct_role().to_name());
        let mut properties = marked.properties();
        properties.identify(mcid);
        if let Some(text) = actual_text {
            properties.actual_text(TextStr(text));
        }
        drop(properties);
        drop(marked);
        self.adopt(element, Kid::Mcid { page, mcid });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened(tagger: &mut Tagger, out: &mut Content, s: Structure) -> Opened {
        tagger.open(out, &Semantics::Structure(s))
    }

    /// A row split across two pages is one `TR` under one `Table`, its
    /// cells resumed by position: on page 2 the number and name cells hold
    /// only furniture, so the text there lands on the third cell's element
    /// and the first two gain nothing.
    #[test]
    fn a_split_row_resumes_across_pages() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        let text = Leaf::Text { actual: None };
        for page in 0..2 {
            tagger.begin_page();
            let table = opened(&mut tagger, &mut out, Structure::Table);
            let row = opened(&mut tagger, &mut out, Structure::Row(7));
            for column in 0..3 {
                let cell = opened(
                    &mut tagger,
                    &mut out,
                    Structure::Cell { column, span: None },
                );
                let furniture = page == 1 && column < 2;
                let repeat = furniture.then(|| tagger.open(&mut out, &Semantics::Pagination));
                let marked = tagger.leaf(&mut out, &text);
                Tagger::end(&mut out, marked);
                if let Some(repeat) = repeat {
                    tagger.close(&mut out, repeat);
                }
                tagger.close(&mut out, cell);
            }
            tagger.close(&mut out, row);
            tagger.close(&mut out, table);
        }
        let tree = tagger.into_tree().expect("a tree");
        let roles: Vec<Role> = tree.elements.iter().map(|e| e.role).collect();
        assert_eq!(
            roles,
            [
                Role::Document,
                Role::Table,
                Role::Tr,
                Role::Td(None),
                Role::Td(None),
                Role::Td(None)
            ]
        );
        let kids = |e: usize| {
            tree.elements
                .get(e)
                .map(|e| e.kids.clone())
                .unwrap_or_default()
        };
        assert_eq!(kids(3), [Kid::Mcid { page: 0, mcid: 0 }]);
        assert_eq!(kids(4), [Kid::Mcid { page: 0, mcid: 1 }]);
        assert_eq!(
            kids(5),
            [
                Kid::Mcid { page: 0, mcid: 2 },
                Kid::Mcid { page: 1, mcid: 0 }
            ]
        );
        assert_eq!(tree.parents, [vec![3, 4, 5], vec![5]]);
    }

    /// rg-969: an element whose content drew nothing is left out, a list
    /// emptied by it too, and the rest renumber; an empty cell stays, so
    /// its row keeps its count.
    #[test]
    fn elements_with_no_content_are_pruned() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        let text = Leaf::Text { actual: None };
        tagger.begin_page();
        let list = opened(&mut tagger, &mut out, Structure::List(ListKey::Key));
        let item = opened(&mut tagger, &mut out, Structure::ListItem);
        tagger.close(&mut out, item);
        tagger.close(&mut out, list);
        let heading = tagger.open(&mut out, &Semantics::Heading(NonZeroU8::MIN));
        let marked = tagger.leaf(&mut out, &text);
        Tagger::end(&mut out, marked);
        tagger.close(&mut out, heading);
        let table = opened(&mut tagger, &mut out, Structure::Table);
        let row = opened(&mut tagger, &mut out, Structure::Row(0));
        for column in 0..2 {
            let cell = opened(
                &mut tagger,
                &mut out,
                Structure::Cell { column, span: None },
            );
            if column == 1 {
                let marked = tagger.leaf(&mut out, &text);
                Tagger::end(&mut out, marked);
            }
            tagger.close(&mut out, cell);
        }
        tagger.close(&mut out, row);
        tagger.close(&mut out, table);
        let tree = tagger.into_tree().expect("a tree");
        let roles: Vec<Role> = tree.elements.iter().map(|e| e.role).collect();
        assert_eq!(
            roles,
            [
                Role::Document,
                Role::Heading(NonZeroU8::MIN),
                Role::Table,
                Role::Tr,
                Role::Td(None),
                Role::Td(None)
            ]
        );
        let kids = |e: usize| tree.elements.get(e).map(|e| e.kids.clone());
        assert_eq!(kids(0), Some(vec![Kid::Element(1), Kid::Element(2)]));
        assert_eq!(kids(3), Some(vec![Kid::Element(4), Kid::Element(5)]));
        assert_eq!(tree.parents, [vec![1, 5]]);
        let parents: Vec<Option<usize>> = tree.elements.iter().map(|e| e.parent).collect();
        assert_eq!(parents, [None, Some(0), Some(0), Some(2), Some(3), Some(3)]);
    }

    /// rg-969 across pages: a list empty on page 1 and resumed with text
    /// on page 2 is kept, its empty item is not; page 2's MCIDs renumber
    /// to their elements; an item keeps its label though its body is empty.
    #[test]
    fn pruning_spans_pages() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        let text = Leaf::Text { actual: None };
        let write = |tagger: &mut Tagger, out: &mut Content| {
            let marked = tagger.leaf(out, &text);
            Tagger::end(out, marked);
        };
        tagger.begin_page();
        let list = opened(&mut tagger, &mut out, Structure::List(ListKey::Key));
        let item = opened(&mut tagger, &mut out, Structure::ListItem);
        tagger.close(&mut out, item);
        tagger.close(&mut out, list);
        let heading = tagger.open(&mut out, &Semantics::Heading(NonZeroU8::MIN));
        write(&mut tagger, &mut out);
        tagger.close(&mut out, heading);
        tagger.begin_page();
        let list = opened(&mut tagger, &mut out, Structure::List(ListKey::Key));
        for labelled in [false, true] {
            let item = opened(&mut tagger, &mut out, Structure::ListItem);
            if labelled {
                let label = opened(&mut tagger, &mut out, Structure::Label);
                write(&mut tagger, &mut out);
                tagger.close(&mut out, label);
            }
            let body = opened(&mut tagger, &mut out, Structure::Body);
            if !labelled {
                write(&mut tagger, &mut out);
            }
            tagger.close(&mut out, body);
            tagger.close(&mut out, item);
        }
        tagger.close(&mut out, list);
        let tree = tagger.into_tree().expect("a tree");
        let roles: Vec<Role> = tree.elements.iter().map(|e| e.role).collect();
        assert_eq!(
            roles,
            [
                Role::Document,
                Role::L,
                Role::Heading(NonZeroU8::MIN),
                Role::Li,
                Role::LBody,
                Role::Li,
                Role::Lbl
            ]
        );
        let kids = |e: usize| tree.elements.get(e).map(|e| e.kids.clone());
        assert_eq!(kids(1), Some(vec![Kid::Element(3), Kid::Element(5)]));
        assert_eq!(kids(5), Some(vec![Kid::Element(6)]));
        assert_eq!(tree.parents, [vec![2], vec![4, 6]]);
    }

    /// rg-2qs: a figure keeps the box it was given; a table given one on
    /// each of two pages keeps none; any other element ignores one.
    #[test]
    fn only_a_one_page_figure_or_table_keeps_a_box() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        let text = Leaf::Text { actual: None };
        let bbox = [1.0, 2.0, 3.0, 4.0];
        for page in 0..2 {
            tagger.begin_page();
            if page == 0 {
                let figure = tagger.open(&mut out, &Semantics::Group("Radar".to_owned()));
                tagger.extent(figure, bbox);
                let marked = tagger.leaf(&mut out, &Leaf::Shape);
                Tagger::end(&mut out, marked);
                tagger.close(&mut out, figure);
                let heading = tagger.open(&mut out, &Semantics::Heading(NonZeroU8::MIN));
                tagger.extent(heading, bbox);
                let marked = tagger.leaf(&mut out, &text);
                Tagger::end(&mut out, marked);
                tagger.close(&mut out, heading);
            }
            let table = opened(&mut tagger, &mut out, Structure::Table);
            tagger.extent(table, bbox);
            let cell = opened(
                &mut tagger,
                &mut out,
                Structure::Cell {
                    column: 0,
                    span: None,
                },
            );
            let marked = tagger.leaf(&mut out, &text);
            Tagger::end(&mut out, marked);
            tagger.close(&mut out, cell);
            tagger.close(&mut out, table);
        }
        let tree = tagger.into_tree().expect("a tree");
        let boxes: Vec<(Role, Option<[f32; 4]>)> =
            tree.elements.iter().map(|e| (e.role, e.bbox)).collect();
        assert_eq!(
            boxes,
            [
                (Role::Document, None),
                (Role::Figure, Some(bbox)),
                (Role::Heading(NonZeroU8::MIN), None),
                (Role::Table, None),
                (Role::Td(None), None),
                (Role::Td(None), None),
            ]
        );
    }

    /// rg-2qs with rg-969: a box stays on its figure or table when an
    /// element before it is pruned, and a header cell after it gains none.
    #[test]
    fn a_box_survives_pruning_before_it() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        let text = Leaf::Text { actual: None };
        let bbox = [1.0, 2.0, 3.0, 4.0];
        tagger.begin_page();
        for _ in 0..2 {
            let list = opened(&mut tagger, &mut out, Structure::List(ListKey::Key));
            let item = opened(&mut tagger, &mut out, Structure::ListItem);
            tagger.close(&mut out, item);
            tagger.close(&mut out, list);
        }
        let figure = tagger.open(&mut out, &Semantics::Group("Radar".to_owned()));
        tagger.extent(figure, bbox);
        let marked = tagger.leaf(&mut out, &Leaf::Shape);
        Tagger::end(&mut out, marked);
        tagger.close(&mut out, figure);
        let table = opened(&mut tagger, &mut out, Structure::Table);
        tagger.extent(table, bbox);
        let row = opened(&mut tagger, &mut out, Structure::HeaderRow);
        let header = opened(&mut tagger, &mut out, Structure::Header(Scope::Column));
        let marked = tagger.leaf(&mut out, &text);
        Tagger::end(&mut out, marked);
        tagger.close(&mut out, header);
        tagger.close(&mut out, row);
        tagger.close(&mut out, table);
        let tree = tagger.into_tree().expect("a tree");
        let boxes: Vec<(Role, Option<[f32; 4]>)> =
            tree.elements.iter().map(|e| (e.role, e.bbox)).collect();
        assert_eq!(
            boxes,
            [
                (Role::Document, None),
                (Role::Figure, Some(bbox)),
                (Role::Table, Some(bbox)),
                (Role::Tr, None),
                (Role::Th(Scope::Column), None),
            ]
        );
    }

    /// rg-969: a count past what the PDF can number is the tree's error,
    /// so the file is not written.
    #[test]
    fn too_many_marks_is_an_error() {
        let mut tagger = Tagger::new();
        tagger.too_large = Some(Fatal::PdfTooLarge {
            what: "marked sequences on one page",
            count: 1 << 31,
        });
        assert!(matches!(
            tagger.into_tree(),
            Err(Fatal::PdfTooLarge { count, .. }) if count == 1 << 31
        ));
    }

    /// Nothing inside an artifact is marked or opens an element.
    #[test]
    fn an_artifact_is_opaque() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        tagger.begin_page();
        let artifact = tagger.open(&mut out, &Semantics::Pagination);
        let inner = tagger.open(&mut out, &Semantics::Heading(NonZeroU8::MIN));
        assert_eq!(inner, Opened::Nothing);
        let nested = tagger.open(&mut out, &Semantics::Decorative);
        assert_eq!(nested, Opened::Nothing);
        assert_eq!(
            tagger.leaf(&mut out, &Leaf::Text { actual: None }),
            Marked(false)
        );
        tagger.close(&mut out, artifact);
        assert_eq!(tagger.into_tree().expect("a tree").elements.len(), 1);
    }

    /// Text no element claims is a paragraph of its own; inside the figure
    /// a shape is the figure's, elsewhere an artifact.
    #[test]
    fn stray_text_is_a_paragraph_and_shapes_are_artifacts() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        tagger.begin_page();
        let marked = tagger.leaf(&mut out, &Leaf::Text { actual: None });
        Tagger::end(&mut out, marked);
        let marked = tagger.leaf(&mut out, &Leaf::Shape);
        Tagger::end(&mut out, marked);
        let figure = tagger.open(&mut out, &Semantics::Group("Technology radar".to_owned()));
        let marked = tagger.leaf(&mut out, &Leaf::Shape);
        Tagger::end(&mut out, marked);
        tagger.close(&mut out, figure);
        let tree = tagger.into_tree().expect("a tree");
        let p = tree.elements.get(1).expect("a paragraph");
        assert_eq!((p.role, p.kids.len()), (Role::P, 1));
        let figure = tree.elements.get(2).expect("the figure");
        assert_eq!(figure.role, Role::Figure);
        assert_eq!(figure.alt.as_deref(), Some("Technology radar"));
        assert_eq!(figure.kids, [Kid::Mcid { page: 0, mcid: 1 }]);
    }
}
