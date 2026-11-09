//! The palette entry that puts an annotation on the open PDF.
//!
//! `PDF_ANNOTATE_ADD` is Silica's own id — the `silica:` prefix keeps it
//! clear of every id Obsidian registers now or later — and [`AddRequest`]
//! is the payload it carries. The app wires the id to
//! [`crate::AnnotationStore::add`]; everything the command needs about the
//! document itself, the page it lands on and where the ink goes, arrives in
//! the request from the view that has the selection.

use crate::annotate::{Kind, Rect};

/// The command id that adds an annotation to the open PDF.
pub const PDF_ANNOTATE_ADD: &str = "silica:pdf-annotate-add";

/// What the command carries: which page, what to draw, where, and who drew
/// it. A highlight over the current selection carries no author; a comment
/// carries its text in [`Kind::Text`].
#[derive(Debug, Clone, PartialEq)]
pub struct AddRequest {
    /// Zero-based page index, as the view counts pages.
    pub page: usize,
    pub kind: Kind,
    /// The rectangle in PDF user space (points), origin bottom-left.
    pub rect: Rect,
    pub author: Option<String>,
}

impl AddRequest {
    /// A highlight over `rect` on `page`.
    pub fn highlight(page: usize, rect: Rect) -> Self {
        Self {
            page,
            kind: Kind::Highlight,
            rect,
            author: None,
        }
    }

    /// An underline over `rect` on `page`.
    pub fn underline(page: usize, rect: Rect) -> Self {
        Self {
            page,
            kind: Kind::Underline,
            rect,
            author: None,
        }
    }

    /// A note with `contents` attached to `rect` on `page`.
    pub fn comment(page: usize, rect: Rect, contents: impl Into<String>) -> Self {
        Self {
            page,
            kind: Kind::Text {
                contents: contents.into(),
            },
            rect,
            author: None,
        }
    }

    /// The request with `author` filled in.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect {
            x0: 1.0,
            y0: 2.0,
            x1: 3.0,
            y1: 4.0,
        }
    }

    #[test]
    fn the_id_keeps_the_silica_prefix() {
        assert_eq!(PDF_ANNOTATE_ADD, "silica:pdf-annotate-add");
        assert!(
            PDF_ANNOTATE_ADD.starts_with("silica:"),
            "a Silica id never collides with an Obsidian id"
        );
    }

    #[test]
    fn a_highlight_carries_no_author_or_text() {
        let request = AddRequest::highlight(2, rect());
        assert_eq!(request.page, 2);
        assert_eq!(request.kind, Kind::Highlight);
        assert_eq!(request.rect, rect());
        assert_eq!(request.author, None);
    }

    #[test]
    fn a_comment_carries_its_text_in_the_kind() {
        let request = AddRequest::comment(0, rect(), "read again").with_author("alice");
        assert_eq!(
            request.kind,
            Kind::Text {
                contents: "read again".into()
            }
        );
        assert_eq!(request.author.as_deref(), Some("alice"));
    }

    #[test]
    fn an_underline_from_the_request_builder() {
        assert_eq!(AddRequest::underline(1, rect()).kind, Kind::Underline);
    }
}
