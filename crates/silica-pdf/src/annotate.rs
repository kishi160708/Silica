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
//!
//! Keeping annotations across sessions is [`crate::store`]'s job: one
//! sidecar JSON file per PDF, which rebuilds a layer through
//! [`AnnotationLayer::restored`]. Painting them again after a repaint is
//! [`Overlay`]: it maps every rectangle onto the pixel space of the rendered
//! page bitmap, so the view draws ink without asking the PDF for anything.

use crate::PageSize;
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

    /// An id read back from a sidecar file. The store is the only caller;
    /// hand it a whole number the file actually carried.
    pub(crate) fn from_raw(value: u64) -> Self {
        Self(value)
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

    /// A layer rebuilt from data that was saved earlier — the sidecar file's
    /// annotations, in the order they were stored. Ids keep the values they
    /// were saved with, and the mint continues above the highest one seen,
    /// so a reload never hands out an id that is still on the page.
    pub fn restored(items: Vec<Annotation>) -> Self {
        let next_id = items.iter().map(|item| item.id.as_u64()).max().unwrap_or(0) + 1;
        Self { next_id, items }
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

/// A rectangle in a rendered page bitmap's pixel space: origin top-left,
/// y down — the space a bitmap is drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PixelRect {
    /// True when `(x, y)` lies inside or on the edge of the rectangle.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    /// True when the rectangle covers no pixel at all.
    pub fn is_degenerate(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

/// One annotation placed on a rendered page, ready to paint.
#[derive(Debug, Clone, PartialEq)]
pub struct Overlay {
    pub id: AnnotationId,
    pub page: usize,
    pub kind: Kind,
    pub area: PixelRect,
}

/// Map a user-space rectangle onto a page of `size` points drawn at
/// `width` x `height` pixels. `None` when the mapping has no usable frame —
/// a page with no positive finite size, a zero-sized bitmap, or coordinates
/// that are not finite — so a bad page size never paints NaN.
fn pixel_area(rect: Rect, size: PageSize, width: u32, height: u32) -> Option<PixelRect> {
    let finite = rect.x0.is_finite()
        && rect.y0.is_finite()
        && rect.x1.is_finite()
        && rect.y1.is_finite()
        && size.width.is_finite()
        && size.height.is_finite();
    if !finite || size.width <= 0.0 || size.height <= 0.0 || width == 0 || height == 0 {
        return None;
    }
    let (pixels_x, pixels_y) = (f64::from(width), f64::from(height));
    let scale_x = pixels_x / f64::from(size.width);
    let scale_y = pixels_y / f64::from(size.height);
    let left = (f64::from(rect.x0) * scale_x).clamp(0.0, pixels_x);
    let right = (f64::from(rect.x1) * scale_x).clamp(0.0, pixels_x);
    // User space has y up; a bitmap has y down, so the top edge is the
    // larger user-space coordinate.
    let top = (pixels_y - f64::from(rect.y1) * scale_y).clamp(0.0, pixels_y);
    let bottom = (pixels_y - f64::from(rect.y0) * scale_y).clamp(0.0, pixels_y);
    let area = PixelRect {
        x: left as f32,
        y: top as f32,
        width: (right - left) as f32,
        height: (bottom - top) as f32,
    };
    (!area.is_degenerate()).then_some(area)
}

impl Annotation {
    /// The annotation mapped onto a page of `size` points drawn at
    /// `width` x `height` pixels, or `None` when the mapping covers no
    /// pixel. Rectangles reaching past the page are clipped to the bitmap.
    pub fn pixel_area(&self, size: PageSize, width: u32, height: u32) -> Option<PixelRect> {
        pixel_area(self.rect, size, width, height)
    }
}

impl AnnotationLayer {
    /// Every annotation of `page` mapped onto the rendered bitmap, in
    /// insertion order — the list a repaint draws.
    pub fn overlay(&self, page: usize, size: PageSize, width: u32, height: u32) -> Vec<Overlay> {
        self.items
            .iter()
            .filter(|item| item.page == page)
            .filter_map(|item| {
                item.pixel_area(size, width, height).map(|area| Overlay {
                    id: item.id,
                    page: item.page,
                    kind: item.kind.clone(),
                    area,
                })
            })
            .collect()
    }

    /// The annotations of `page` whose mapped area contains `(x, y)`,
    /// topmost (last inserted) first — pixel-space hit testing over a
    /// rendered page.
    pub fn overlay_at(
        &self,
        page: usize,
        size: PageSize,
        width: u32,
        height: u32,
        x: f32,
        y: f32,
    ) -> Vec<Overlay> {
        let mut found = self.overlay(page, size, width, height);
        found.retain(|item| item.area.contains(x, y));
        found.reverse();
        found
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

    #[test]
    fn restored_keeps_saved_ids_and_mints_above_them() {
        let mut first = AnnotationLayer::new();
        let one = first.add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None);
        let two = first.add(1, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None);
        let saved: Vec<Annotation> = first.iter().cloned().collect();

        let mut second = AnnotationLayer::restored(saved);
        assert_eq!(second.annotation(one).map(|item| item.id), Some(one));
        assert!(second.remove(two));
        let three = second.add(2, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None);
        assert_eq!(three.as_u64(), two.as_u64() + 1);
        assert_eq!(second.len(), 2);
        assert_eq!(second.annotation(three).unwrap().page, 2);
    }

    #[test]
    fn restored_of_nothing_mints_from_one() {
        let mut layer = AnnotationLayer::restored(Vec::new());
        let id = layer.add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None);
        assert_eq!(id.as_u64(), 1);
    }

    #[test]
    fn pixel_area_flips_y_and_fills_the_bitmap() {
        let size = PageSize {
            width: 100.0,
            height: 200.0,
        };
        let mut layer = AnnotationLayer::new();
        let full = layer.add(0, Kind::Highlight, rect(0.0, 0.0, 100.0, 200.0), None);
        assert_eq!(
            layer.annotation(full).unwrap().pixel_area(size, 50, 100),
            Some(PixelRect {
                x: 0.0,
                y: 0.0,
                width: 50.0,
                height: 100.0
            })
        );

        // The top half in user space is the top half of the bitmap.
        let top = layer.add(0, Kind::Highlight, rect(0.0, 100.0, 100.0, 200.0), None);
        assert_eq!(
            layer.annotation(top).unwrap().pixel_area(size, 50, 100),
            Some(PixelRect {
                x: 0.0,
                y: 0.0,
                width: 50.0,
                height: 50.0
            })
        );

        // The bottom half in user space sits at the bottom of the bitmap.
        let bottom = layer.add(1, Kind::Highlight, rect(0.0, 0.0, 100.0, 100.0), None);
        assert_eq!(
            layer.annotation(bottom).unwrap().pixel_area(size, 50, 100),
            Some(PixelRect {
                x: 0.0,
                y: 50.0,
                width: 50.0,
                height: 50.0
            })
        );
    }

    #[test]
    fn pixel_area_clips_and_refuses_unusable_frames() {
        let size = PageSize {
            width: 100.0,
            height: 100.0,
        };
        let mut layer = AnnotationLayer::new();

        // Reaching past the right edge is clipped to the bitmap, not dropped.
        let wide = layer.add(0, Kind::Highlight, rect(80.0, 0.0, 400.0, 100.0), None);
        assert_eq!(
            layer.annotation(wide).unwrap().pixel_area(size, 50, 25),
            Some(PixelRect {
                x: 40.0,
                y: 0.0,
                width: 10.0,
                height: 25.0
            })
        );

        // A rectangle with no area on the page paints nothing.
        let sliver = layer.add(0, Kind::Highlight, rect(10.0, 10.0, 10.0, 90.0), None);
        assert_eq!(
            layer.annotation(sliver).unwrap().pixel_area(size, 50, 25),
            None
        );

        let page = PageSize {
            width: 100.0,
            height: 100.0,
        };
        let id = layer.add(0, Kind::Highlight, rect(0.0, 0.0, 100.0, 100.0), None);
        let annotation = layer.annotation(id).unwrap();
        assert_eq!(
            annotation.pixel_area(
                PageSize {
                    width: 0.0,
                    height: 100.0
                },
                50,
                25
            ),
            None
        );
        assert_eq!(
            annotation.pixel_area(
                PageSize {
                    width: f32::NAN,
                    height: 100.0
                },
                50,
                25
            ),
            None
        );
        assert_eq!(annotation.pixel_area(page, 0, 25), None);

        let nan = layer.add(0, Kind::Highlight, rect(f32::NAN, 0.0, 10.0, 10.0), None);
        assert_eq!(
            layer.annotation(nan).unwrap().pixel_area(page, 50, 25),
            None
        );
    }

    #[test]
    fn overlay_and_pixel_hit_test_follow_the_bitmap() {
        let doc = crate::Document::open(
            std::fs::read(format!(
                "{}/tests/fixtures/pages.pdf",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        )
        .unwrap();
        let size = doc.page_size(0).unwrap();
        let mut layer = AnnotationLayer::new();
        let back = layer.add(
            0,
            Kind::Highlight,
            rect(0.0, 0.0, size.width, size.height),
            None,
        );
        let front = layer.add(
            0,
            Kind::Text {
                contents: "note".into(),
            },
            rect(
                size.width / 2.0 - 5.0,
                size.height / 2.0 - 5.0,
                size.width / 2.0 + 5.0,
                size.height / 2.0 + 5.0,
            ),
            None,
        );
        layer.add(
            1,
            Kind::Highlight,
            rect(0.0, 0.0, size.width, size.height),
            None,
        );

        let (width, height) = (200u32, 400u32);
        let overlay = layer.overlay(0, size, width, height);
        assert_eq!(overlay.len(), 2, "insertion order, page 1 excluded");
        assert_eq!(overlay[0].id, back);
        assert_eq!(overlay[1].id, front);

        // Middle of the bitmap hits the front annotation first.
        let center = (width as f32 / 2.0, height as f32 / 2.0);
        let hit = layer.overlay_at(0, size, width, height, center.0, center.1);
        let hit_ids: Vec<AnnotationId> = hit.iter().map(|item| item.id).collect();
        assert_eq!(hit_ids, vec![front, back]);

        // The top-left corner of the bitmap only holds the page-wide one.
        let corner = layer.overlay_at(0, size, width, height, 1.0, 1.0);
        let corner_ids: Vec<AnnotationId> = corner.iter().map(|item| item.id).collect();
        assert_eq!(corner_ids, vec![back]);

        // A page the document does not have: nothing, at any pixel.
        let missing = doc.page_count() + 5;
        assert!(layer
            .overlay_at(missing, size, width, height, center.0, center.1)
            .is_empty());
    }
}
