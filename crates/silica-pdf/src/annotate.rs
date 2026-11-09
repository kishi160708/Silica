//! PDF annotations: a memory layer on top of [`crate::Document`].
//!
//! Every annotation belongs to one page and carries a rectangle in PDF user
//! space (points, origin bottom-left, y up). The layer assigns a
//! monotonically increasing [`AnnotationId`] to every inserted annotation and
//! keeps insertion order, so the view can paint annotations on top of the
//! rendered page bitmap and hit-test them against a pointer.
//!
//! The layer stores annotations only: it never touches the PDF bytes, never
//! renders, and never validates that `page` is inside the document — the
//! caller supplies page indices from [`Document::page_count`].

use std::fmt;

/// A point in PDF user space (points), origin bottom-left, y up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// A rectangle in PDF user space (points), origin bottom-left, y up.
/// Callers satisfy `x0 <= x1` and `y0 <= y1` when constructing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// True when `point` lies inside or on the edge of the rectangle.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x0 && point.x <= self.x1 && point.y >= self.y0 && point.y <= self.y1
    }

    /// True when the rectangles overlap with positive area.
    pub fn intersects(&self, other: Rect) -> bool {
        self.x0 < other.x1 && self.x1 > other.x0 && self.y0 < other.y1 && self.y1 > other.y0
    }
}

/// The kind of annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// A highlighted region of text.
    Highlight,
    /// An underlined region of text.
    Underline,
    /// A squiggly underline.
    Squiggly,
    /// A struck-out region of text.
    StrikeOut,
    /// A note attached to a region, with free text.
    Text { contents: String },
}

/// One annotation, owned by the layer.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub id: AnnotationId,
    /// Zero-based page index.
    pub page: usize,
    pub kind: Kind,
    pub rect: Rect,
    pub author: Option<String>,
}

/// Opaque annotation handle. Only the layer mints ids; the value is stable
/// for as long as the annotation stays in the layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnnotationId(u64);

impl AnnotationId {
    /// The raw counter value, exposed for tests and log messages.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for AnnotationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// In-memory annotation layer: one flat list, insertion ordered.
pub struct AnnotationLayer {
    next_id: u64,
    items: Vec<Annotation>,
}

impl Default for AnnotationLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnotationLayer {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            items: Vec::new(),
        }
    }

    /// Insert `kind` on `page` inside `rect`, with optional `author`, and
    /// return the new annotation's id.
    pub fn add(
        &mut self,
        page: usize,
        kind: Kind,
        rect: Rect,
        author: Option<String>,
    ) -> AnnotationId {
        let id = AnnotationId(self.next_id);
        self.next_id += 1;
        self.items.push(Annotation {
            id,
            page,
            kind,
            rect,
            author,
        });
        id
    }

    /// Remove the annotation with `id`. Returns `false` when `id` is unknown
    /// (including after an earlier removal).
    pub fn remove(&mut self, id: AnnotationId) -> bool {
        if let Some(pos) = self.items.iter().position(|item| item.id == id) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    /// The annotation with `id`, or `None`.
    pub fn annotation(&self, id: AnnotationId) -> Option<&Annotation> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Every annotation of `page`, in insertion order.
    pub fn page(&self, page: usize) -> Vec<&Annotation> {
        self.items
            .iter()
            .filter(move |item| item.page == page)
            .collect()
    }

    /// Every annotation of `page` whose rect contains `point`, topmost
    /// (last inserted) first.
    pub fn at(&self, page: usize, point: Point) -> Vec<&Annotation> {
        let mut found: Vec<&Annotation> = self
            .items
            .iter()
            .filter(|item| item.page == page && item.rect.contains(point))
            .collect();
        found.reverse();
        found
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.items.iter(),
        }
    }
}

/// Iteration over a layer's annotations, oldest first.
pub struct Iter<'a> {
    inner: std::slice::Iter<'a, Annotation>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Annotation;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    #[test]
    fn ids_increase_and_stick() {
        let mut layer = AnnotationLayer::new();
        let one = layer.add(0, Kind::Highlight, rect(0.0, 0.0, 10.0, 10.0), None);
        let two = layer.add(1, Kind::Underline, rect(100.0, 100.0, 110.0, 110.0), None);
        assert_eq!(two.as_u64(), one.as_u64() + 1);
        assert_eq!(layer.annotation(one).unwrap().page, 0);
        assert_eq!(layer.annotation(two).unwrap().kind, Kind::Underline);
    }

    #[test]
    fn remove_unknown_or_twice_is_false() {
        let mut layer = AnnotationLayer::new();
        let id = layer.add(
            0,
            Kind::Text {
                contents: "note".into(),
            },
            rect(0.0, 0.0, 5.0, 5.0),
            Some("alice".into()),
        );
        assert!(layer.remove(id));
        assert!(!layer.remove(id));
        assert!(!layer.remove(AnnotationId(999)));
        assert!(layer.is_empty());
    }

    #[test]
    fn page_returns_insertion_order() {
        let mut layer = AnnotationLayer::new();
        let a = layer.add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None);
        let b = layer.add(1, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None);
        let c = layer.add(0, Kind::Underline, rect(0.0, 0.0, 1.0, 1.0), None);
        let on_first: Vec<AnnotationId> = layer.page(0).iter().map(|ann| ann.id).collect();
        assert_eq!(on_first, vec![a, c]);
        assert_eq!(layer.page(1).len(), 1);
        assert_eq!(layer.page(1)[0].id, b);
        assert_eq!(layer.page(42).len(), 0);
    }

    #[test]
    fn at_hit_test_is_topmost_first() {
        let mut layer = AnnotationLayer::new();
        let back = layer.add(0, Kind::Highlight, rect(0.0, 0.0, 20.0, 20.0), None);
        let front = layer.add(0, Kind::Underline, rect(5.0, 5.0, 15.0, 15.0), None);
        layer.add(1, Kind::Highlight, rect(100.0, 100.0, 120.0, 120.0), None);

        let hit: Vec<AnnotationId> = layer
            .at(0, Point { x: 10.0, y: 10.0 })
            .iter()
            .map(|ann| ann.id)
            .collect();
        assert_eq!(hit, vec![front, back]);

        // Outside every rect on that page.
        assert!(layer.at(0, Point { x: 100.0, y: 100.0 }).is_empty());
        // Same coordinates on another page: nothing.
        assert!(layer.at(1, Point { x: 10.0, y: 10.0 }).is_empty());
    }

    #[test]
    fn rect_contains_and_intersects() {
        let r = rect(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(Point { x: 0.0, y: 0.0 }));
        assert!(r.contains(Point { x: 10.0, y: 10.0 }));
        assert!(!r.contains(Point { x: -0.1, y: 5.0 }));
        assert!(!r.contains(Point { x: 5.0, y: 10.1 }));

        let inside = rect(2.0, 2.0, 4.0, 4.0);
        let beside = rect(10.0, 0.0, 20.0, 10.0);
        assert!(r.intersects(inside));
        assert!(!r.intersects(beside));
    }

    #[test]
    fn iter_is_oldest_first() {
        let mut layer = AnnotationLayer::new();
        let ids: Vec<AnnotationId> = (0..3)
            .map(|_| layer.add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None))
            .collect();
        let seen: Vec<AnnotationId> = layer.iter().map(|ann| ann.id).collect();
        assert_eq!(seen, ids);
    }

    #[test]
    fn layer_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AnnotationLayer>();
    }

    #[test]
    fn annotation_pages_fit_inside_a_real_document() {
        let path = format!("{}/tests/fixtures/pages.pdf", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        let doc = crate::Document::open(bytes).unwrap();
        let mut layer = AnnotationLayer::new();
        for index in 0..doc.page_count() {
            layer.add(index, Kind::Highlight, rect(0.0, 0.0, 10.0, 10.0), None);
        }
        assert_eq!(layer.len(), doc.page_count());
        for index in 0..doc.page_count() {
            assert_eq!(layer.page(index).len(), 1);
        }
        assert_eq!(layer.page(doc.page_count()).len(), 0);
    }
}
