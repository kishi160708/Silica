//! Round-trip check: annotations attached to pages of a real fixture PDF,
//! then removed, with the layer staying consistent.

use silica_pdf::{AnnotationId, AnnotationLayer, Document, Kind, Rect};

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading {path}: {e}"))
}

fn middle(size: silica_pdf::PageSize) -> Rect {
    Rect {
        x0: size.width / 2.0 - 20.0,
        y0: size.height / 2.0 - 20.0,
        x1: size.width / 2.0 + 20.0,
        y1: size.height / 2.0 + 20.0,
    }
}

#[test]
fn one_highlight_per_page_of_pages_pdf() {
    let doc = Document::open(fixture("pages.pdf")).unwrap();
    let mut layer = AnnotationLayer::new();
    for index in 0..doc.page_count() {
        layer.add(
            index,
            Kind::Highlight,
            middle(doc.page_size(index).unwrap()),
            None,
        );
    }
    assert_eq!(layer.len(), doc.page_count());
    for index in 0..doc.page_count() {
        let annots = layer.page(index);
        assert_eq!(annots.len(), 1, "page {index} must carry one annotation");
        assert_eq!(annots[0].kind, Kind::Highlight);
    }
}

#[test]
fn removal_leaves_the_rest_intact() {
    let doc = Document::open(fixture("pages.pdf")).unwrap();
    let mut layer = AnnotationLayer::new();
    for index in 0..doc.page_count() {
        layer.add(
            index,
            Kind::Underline,
            middle(doc.page_size(index).unwrap()),
            None,
        );
    }
    let id = layer.page(0)[0].id;
    assert!(layer.remove(id));
    assert_eq!(layer.page(0).len(), 0);
    assert_eq!(layer.len(), doc.page_count() - 1);
    for index in 1..doc.page_count() {
        assert_eq!(layer.page(index).len(), 1);
    }
}

#[test]
fn hit_test_uses_document_page_space() {
    let doc = Document::open(fixture("pages.pdf")).unwrap();
    let mut layer = AnnotationLayer::new();
    for index in 0..doc.page_count() {
        let size = doc.page_size(index).unwrap();
        layer.add(index, Kind::Highlight, middle(size), Some("qa".into()));
    }
    let size = doc.page_size(1).unwrap();
    let center = silica_pdf::Point {
        x: size.width / 2.0,
        y: size.height / 2.0,
    };
    let hit: Vec<AnnotationId> = layer.at(1, center).iter().map(|ann| ann.id).collect();
    assert_eq!(hit.len(), 1);
    assert_eq!(
        layer.annotation(hit[0]).unwrap().author.as_deref(),
        Some("qa")
    );

    // Outside the page's rect: nothing, even at the same coordinates on another page.
    let corner = silica_pdf::Point { x: 0.0, y: 0.0 };
    assert!(layer.at(1, corner).is_empty());
}
