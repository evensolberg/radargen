//! The structure tree's objects (FR-114): the root, every element in the
//! tagger's order, and the parent tree, one array per page mapping its
//! MCIDs to their elements. Numbered after the fonts, in that order, so
//! the file is a function of the document (NFR-4).

use pdf_writer::types::TableHeaderScope;
use pdf_writer::writers::{StructChildren, StructTreeRoot};
use pdf_writer::{Pdf, Ref, TextStr};

use super::tag::{Element, Kid, Role, Tree};
use super::{PAGES, integer};
use crate::diagnostics::Fatal;
use crate::svg::Scope;

/// Writes `tree` with the root at `root`, numbering its elements and
/// parent-tree arrays from `next`; `pages` are the page objects in order.
///
/// # Errors
///
/// [`Fatal::PdfTooLarge`] when the pages, or a cell's span, are past what
/// a PDF can number.
pub fn write(
    pdf: &mut Pdf,
    tree: &Tree,
    root: Ref,
    next: &mut Ref,
    pages: &[Ref],
) -> Result<(), Fatal> {
    let ids: Vec<Ref> = tree.elements.iter().map(|_| next.bump()).collect();
    let arrays: Vec<Ref> = tree.parents.iter().map(|_| next.bump()).collect();
    root_dict(pdf, root, ids.first().copied(), &arrays)?;
    for (element, &id) in tree.elements.iter().zip(&ids) {
        write_element(pdf, (element, id), (&ids, root), pages)?;
    }
    for (parents, &array) in tree.parents.iter().zip(&arrays) {
        pdf.indirect(array)
            .array()
            .items(parents.iter().filter_map(|&e| ids.get(e).copied()));
    }
    Ok(())
}

/// The root: its one kid, the document, and the parent tree, page `k`'s
/// array keyed `k`.
fn root_dict(pdf: &mut Pdf, root: Ref, document: Option<Ref>, arrays: &[Ref]) -> Result<(), Fatal> {
    let mut dict = pdf.indirect(root).start::<StructTreeRoot>();
    dict.children().items(document);
    let mut tree = dict.parent_tree();
    let mut entries = tree.nums();
    for (page, &array) in arrays.iter().enumerate() {
        entries.insert(integer(page, PAGES)?, array);
    }
    drop(entries);
    drop(tree);
    dict.parent_tree_next_key(integer(arrays.len(), PAGES)?);
    Ok(())
}

/// One element: its role, parent, page, `/Alt`, attributes and kids.
fn write_element(
    pdf: &mut Pdf,
    (element, id): (&Element, Ref),
    (ids, root): (&[Ref], Ref),
    pages: &[Ref],
) -> Result<(), Fatal> {
    let parent = element
        .parent
        .and_then(|p| ids.get(p).copied())
        .unwrap_or(root);
    // The element's own page: its first marked sequence's.
    let first = element.kids.iter().find_map(|kid| match kid {
        Kid::Mcid { page, .. } => Some(*page),
        Kid::Element(_) => None,
    });
    let mut dict = pdf.struct_element(id);
    dict.kind(element.role.struct_role()).parent(parent);
    if let Some(page) = first.and_then(|p| pages.get(p)) {
        dict.page(*page);
    }
    if let Some(alt) = &element.alt {
        dict.alt(TextStr(alt));
    }
    attributes(&mut dict, element.role)?;
    let mut kids = dict.children();
    for kid in &element.kids {
        write_kid(&mut kids, *kid, first, (ids, pages));
    }
    Ok(())
}

/// One kid: an element by reference, an MCID on the element's own page, or
/// a marked-content reference naming another page.
fn write_kid(
    kids: &mut StructChildren<'_>,
    kid: Kid,
    first: Option<usize>,
    (ids, pages): (&[Ref], &[Ref]),
) {
    match kid {
        Kid::Element(e) => {
            if let Some(&child) = ids.get(e) {
                kids.struct_element(child);
            }
        }
        Kid::Mcid { page, mcid } if Some(page) == first => {
            kids.marked_content_id(mcid);
        }
        Kid::Mcid { page, mcid } => {
            if let Some(&on) = pages.get(page) {
                kids.marked_content_ref().marked_content_id(mcid).page(on);
            }
        }
    }
}

/// A header cell's scope; a spanning data cell's span.
fn attributes(dict: &mut pdf_writer::writers::StructElement<'_>, role: Role) -> Result<(), Fatal> {
    match role {
        Role::Th(scope) => {
            dict.attributes().push().table().scope(match scope {
                Scope::Column => TableHeaderScope::Column,
                Scope::Row => TableHeaderScope::Row,
            });
        }
        Role::Td(Some(span)) => {
            dict.attributes()
                .push()
                .table()
                .col_span(integer(span.get(), "columns in one cell")?);
        }
        _ => {}
    }
    Ok(())
}
