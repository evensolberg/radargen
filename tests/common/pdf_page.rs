//! Shared by the tests that read a PDF's drawing: its one page and that
//! page's content operators, through lopdf.

use lopdf::content::{Content, Operation};
use lopdf::{Document, ObjectId};

/// `doc`'s first page.
pub fn first_page(doc: &Document) -> ObjectId {
    *doc.get_pages().values().next().expect("a page")
}

/// The content operators of `pdf`'s first page.
pub fn operations(pdf: &[u8]) -> Vec<Operation> {
    let doc = Document::load_mem(pdf).expect("parses");
    Content::decode(&doc.get_page_content(first_page(&doc)))
        .expect("decodes")
        .operations
}
