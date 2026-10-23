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

use pdf_writer::types::ArtifactType;
use pdf_writer::{Content, Name, TextStr};

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
    /// The role's name, which is also the tag of the marked content under
    /// it.
    #[must_use]
    pub const fn name(self) -> &'static [u8] {
        match self {
            Self::Document => b"Document",
            Self::Heading(level) => match level.get() {
                1 => b"H1",
                2 => b"H2",
                3 => b"H3",
                4 => b"H4",
                5 => b"H5",
                _ => b"H6",
            },
            Self::P => b"P",
            Self::Figure => b"Figure",
            Self::L => b"L",
            Self::Li => b"LI",
            Self::Lbl => b"Lbl",
            Self::LBody => b"LBody",
            Self::Table => b"Table",
            Self::Tr => b"TR",
            Self::Th(_) => b"TH",
            Self::Td(_) => b"TD",
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Its role.
    pub role: Role,
    /// Its parent's index; `None` for the root.
    pub parent: Option<usize>,
    /// Its alternate description: the radar's name (D-75).
    pub alt: Option<String>,
    /// Its children, in reading order.
    pub kids: Vec<Kid>,
}

/// The finished tree: element 0 is the document; each page's parent-tree
/// array maps its MCIDs, in order, to their elements.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            }],
            keys: BTreeMap::new(),
            stack: Vec::new(),
            artifacts: 0,
            figures: 0,
            parents: Vec::new(),
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

    /// The tree, once every page is drawn.
    #[must_use]
    pub fn into_tree(self) -> Tree {
        Tree {
            elements: self.elements,
            parents: self.parents,
        }
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
                self.element(Role::Figure, None, Some(name.clone()))
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
                if let Some(frame) = self.stack.pop()
                    && self.elements.get(frame.element).map(|e| e.role) == Some(Role::Figure)
                {
                    self.figures = self.figures.saturating_sub(1);
                }
            }
            Opened::Artifact => {
                self.artifacts = self.artifacts.saturating_sub(1);
                out.end_marked_content();
            }
        }
    }

    /// Marks a text leaf: an MCID under its nearest element, or under a
    /// paragraph of its own; `actual_text` when its words are not its
    /// drawn lines (D-88). Inside an artifact it is not marked again.
    pub fn text(&mut self, out: &mut Content, actual_text: Option<&str>) -> Marked {
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

    /// Marks a shape leaf: the figure's inside the radar, else an artifact.
    pub fn shape(&mut self, out: &mut Content) -> Marked {
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

    /// Marks a leaf that is always an artifact: the ground, a halo.
    pub fn ornament(&self, out: &mut Content) -> Marked {
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
        let mcid = i32::try_from(parents.len()).unwrap_or(i32::MAX);
        parents.push(element);
        let role = self.elements.get(element).map_or(Role::P, |e| e.role);
        let mut marked = out.begin_marked_content_with_properties(Name(role.name()));
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
    /// cells resumed by position, holding MCIDs on both pages.
    #[test]
    fn a_split_row_resumes_across_pages() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        for _ in 0..2 {
            tagger.begin_page();
            let table = opened(&mut tagger, &mut out, Structure::Table);
            let row = opened(&mut tagger, &mut out, Structure::Row(7));
            for column in 0..2 {
                let cell = opened(
                    &mut tagger,
                    &mut out,
                    Structure::Cell { column, span: None },
                );
                let marked = tagger.text(&mut out, None);
                Tagger::end(&mut out, marked);
                tagger.close(&mut out, cell);
            }
            tagger.close(&mut out, row);
            tagger.close(&mut out, table);
        }
        let tree = tagger.into_tree();
        let roles: Vec<Role> = tree.elements.iter().map(|e| e.role).collect();
        assert_eq!(
            roles,
            [
                Role::Document,
                Role::Table,
                Role::Tr,
                Role::Td(None),
                Role::Td(None)
            ]
        );
        let cell = tree.elements.get(3).expect("a cell");
        assert_eq!(
            cell.kids,
            [
                Kid::Mcid { page: 0, mcid: 0 },
                Kid::Mcid { page: 1, mcid: 0 }
            ]
        );
        assert_eq!(tree.parents, [vec![3, 4], vec![3, 4]]);
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
        assert_eq!(tagger.text(&mut out, None), Marked(false));
        tagger.close(&mut out, artifact);
        assert_eq!(tagger.into_tree().elements.len(), 1);
    }

    /// Text no element claims is a paragraph of its own; inside the figure
    /// a shape is the figure's, elsewhere an artifact.
    #[test]
    fn stray_text_is_a_paragraph_and_shapes_are_artifacts() {
        let mut tagger = Tagger::new();
        let mut out = Content::new();
        tagger.begin_page();
        let marked = tagger.text(&mut out, None);
        Tagger::end(&mut out, marked);
        let marked = tagger.shape(&mut out);
        Tagger::end(&mut out, marked);
        let figure = tagger.open(&mut out, &Semantics::Group("Technology radar".to_owned()));
        let marked = tagger.shape(&mut out);
        Tagger::end(&mut out, marked);
        tagger.close(&mut out, figure);
        let tree = tagger.into_tree();
        let p = tree.elements.get(1).expect("a paragraph");
        assert_eq!((p.role, p.kids.len()), (Role::P, 1));
        let figure = tree.elements.get(2).expect("the figure");
        assert_eq!(figure.role, Role::Figure);
        assert_eq!(figure.alt.as_deref(), Some("Technology radar"));
        assert_eq!(figure.kids, [Kid::Mcid { page: 0, mcid: 1 }]);
    }
}
