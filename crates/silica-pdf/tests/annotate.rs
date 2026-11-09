//! Round-trip check: annotations attached to pages of a real fixture PDF,
//! then removed, with the layer staying consistent. The second half drives
//! the sidecar file the way the app does: saved through the store, read back
//! by a fresh one, and repainted over a rendered bitmap.

use silica_pdf::{
    sidecar_path, AddRequest, Annotation, AnnotationId, AnnotationLayer, AnnotationStore, Document,
    Kind, Point, Rect, PDF_ANNOTATE_ADD,
};

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

/// Minimal temp dir: the crate has no `tempfile` dependency and these tests
/// are the only place a vault is needed.
mod tempdir {
    pub struct TempDir(std::path::PathBuf);

    impl TempDir {
        pub fn new(prefix: &str) -> std::io::Result<TempDir> {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "{prefix}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path)?;
            Ok(TempDir(path))
        }

        pub fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

fn vault() -> tempdir::TempDir {
    tempdir::TempDir::new("silica-pdf-annotate").expect("a temp vault")
}

#[test]
fn the_command_id_is_silicas_own() {
    assert_eq!(PDF_ANNOTATE_ADD, "silica:pdf-annotate-add");
}

#[test]
fn the_store_persists_annotations_for_the_pdf_and_only_them() {
    let root = vault();
    let mut store = AnnotationStore::open(root.path(), "pages.pdf").expect("opens");
    assert_eq!(
        store.path(),
        sidecar_path(root.path(), "pages.pdf").unwrap(),
        "the sidecar sits under .silica/annotations/"
    );

    // What the command would apply, taken from the request builder.
    let doc = Document::open(fixture("pages.pdf")).unwrap();
    let size = doc.page_size(0).unwrap();
    let request = AddRequest::highlight(0, middle(size));
    let highlight = store
        .add(
            request.page,
            request.kind,
            request.rect,
            request.author.clone(),
        )
        .expect("saved");
    let comment = store
        .add(
            1,
            AddRequest::comment(1, middle(doc.page_size(1).unwrap()), "third read").kind,
            middle(doc.page_size(1).unwrap()),
            None,
        )
        .expect("saved");

    // A fresh store over the same vault reads both back, unchanged.
    let reopened = AnnotationStore::open(root.path(), "pages.pdf").expect("reopens");
    let saved: Vec<&Annotation> = reopened.annotations().iter().collect();
    assert_eq!(saved.len(), 2);
    assert_eq!(saved[0].id, highlight);
    assert_eq!(saved[0].kind, Kind::Highlight);
    assert_eq!(saved[0].rect, middle(size));
    assert_eq!(saved[1].id, comment);
    assert_eq!(
        saved[1].kind,
        Kind::Text {
            contents: "third read".into()
        }
    );

    // Removing one and reopening keeps the other and its id.
    let mut reopened = reopened;
    reopened.remove(highlight).expect("removed");
    let after = AnnotationStore::open(root.path(), "pages.pdf").expect("reopens");
    assert_eq!(after.annotations().len(), 1);
    assert_eq!(
        after.annotations().annotation(comment).map(|item| item.id),
        Some(comment)
    );

    // A sidecar for a different PDF never leaks into this one.
    let other = AnnotationStore::open(root.path(), "other.pdf").expect("opens");
    assert!(other.annotations().is_empty());
    assert_ne!(other.path(), after.path());
}

#[test]
fn a_repaint_draws_every_saved_annotation_over_the_bitmap() {
    let root = vault();
    let doc = Document::open(fixture("pages.pdf")).unwrap();
    let size = doc.page_size(1).unwrap();
    let bitmap = doc.render(1, 1.0).expect("renders page 1");

    let mut store = AnnotationStore::open(root.path(), "pages.pdf").expect("opens");
    let back = store
        .add(
            1,
            Kind::Highlight,
            Rect {
                x0: 0.0,
                y0: 0.0,
                x1: size.width,
                y1: size.height,
            },
            None,
        )
        .expect("saved");
    let front = store
        .add(
            1,
            Kind::Text {
                contents: "note".into(),
            },
            middle(size),
            None,
        )
        .expect("saved");
    store
        .add(0, Kind::Highlight, middle(size), None)
        .expect("saved on another page");

    // The repaint asks the store, not the file: the same layer a new store
    // would rebuild.
    let reopened = AnnotationStore::open(root.path(), "pages.pdf").expect("reopens");
    let layer: &AnnotationLayer = reopened.annotations();
    assert_eq!(layer.len(), 3);

    let overlay = layer.overlay(1, size, bitmap.width, bitmap.height);
    assert_eq!(overlay.len(), 2, "page 0's annotation is not on this page");
    assert_eq!(overlay[0].id, back);
    assert_eq!(overlay[1].id, front);
    for item in &overlay {
        assert!(item.area.x >= 0.0 && item.area.y >= 0.0);
        assert!(item.area.x + item.area.width <= bitmap.width as f32);
        assert!(item.area.y + item.area.height <= bitmap.height as f32);
    }

    // A click in the middle of the bitmap picks up the note first.
    let center = Point {
        x: bitmap.width as f32 / 2.0,
        y: bitmap.height as f32 / 2.0,
    };
    let hit: Vec<AnnotationId> = layer
        .overlay_at(1, size, bitmap.width, bitmap.height, center.x, center.y)
        .iter()
        .map(|item| item.id)
        .collect();
    assert_eq!(hit, vec![front, back]);
}
