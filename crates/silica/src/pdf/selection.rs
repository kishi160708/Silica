//! Text selection over a PDF's pages: which glyphs a drag covers, the boxes
//! to highlight, and the text a copy leaves.
//!
//! The view hands this module page glyphs in **page points** and pointers in
//! **logical pixels of the scrolled column**; it answers with
//! [`Selection::boxes`] (logical pixels, same space as the pointer) and
//! [`Selection::text`]. No gpui here, so the arithmetic is testable.
//!
//! A selection is an anchor and a focus, each a (page, glyph) pair, ordered
//! so `start <= end`; a drag backwards or up a page selects the same run as
//! one forwards. Glyph indexes are per page and index into
//! [`PdfGlyph`] order, which is the file's content order — the same order
//! [`Document::page_text`] assembles, so a selection's text reads the way
//! find reads it.

use silica_pdf::PdfGlyph;

/// A point in the selection: page index and glyph index within the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mark {
    pub page: usize,
    pub glyph: usize,
}

/// Which glyphs are selected, from `start` to `end` inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: Mark,
    pub end: Mark,
}

/// A highlighted run on one page, in logical pixels of the page's top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Highlight {
    pub page: usize,
    /// Left, top, width, height.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Selection {
    /// The selection a drag from `anchor` to `focus` makes, in either
    /// direction.
    pub fn new(anchor: Mark, focus: Mark) -> Selection {
        if anchor <= focus {
            Selection {
                start: anchor,
                end: focus,
            }
        } else {
            Selection {
                start: focus,
                end: anchor,
            }
        }
    }

    /// Every glyph the selection covers on `page`, as an index range.
    /// `None` when the page is outside it.
    fn range(&self, page: usize, count: usize) -> Option<(usize, usize)> {
        if page < self.start.page || page > self.end.page || count == 0 {
            return None;
        }
        let first = if page == self.start.page {
            self.start.glyph.min(count.saturating_sub(1))
        } else {
            0
        };
        let last = if page == self.end.page {
            self.end.glyph.min(count.saturating_sub(1))
        } else {
            count - 1
        };
        (first <= last).then_some((first, last))
    }

    /// The highlight boxes for the pages in `pages`, given each page's glyphs
    /// and the layout scale (logical pixels per point). Adjacent glyphs on
    /// one line merge into one box, so a line of text is one band rather than
    /// a hundred squares.
    pub fn boxes(&self, pages: &[(usize, &[PdfGlyph])], scale: f32) -> Vec<Highlight> {
        let mut out = Vec::new();
        for (page, glyphs) in pages {
            let Some((first, last)) = self.range(*page, glyphs.len()) else {
                continue;
            };
            let run = &glyphs[first..=last];
            let mut iter = run.iter().enumerate();
            let mut band: Option<(f32, f32, f32, f32)> = None; // x0, x1, y0, y1
            for (_, glyph) in iter.by_ref() {
                let (x0, x1) = (glyph.x * scale, (glyph.x + glyph.width) * scale);
                let (y0, y1) = (glyph.y * scale, (glyph.y + glyph.height) * scale);
                match band {
                    Some((bx0, bx1, by0, by1)) => {
                        // Same line: the boxes overlap vertically by half a
                        // glyph or more. Otherwise the band is done.
                        let overlap = by1.min(y1) - by0.max(y0);
                        let shortest = (by1 - by0).min(y1 - y0).max(0.1);
                        if overlap > shortest * 0.5 && x0 <= bx1 + shortest {
                            band = Some((bx0, bx1.max(x1), by0.min(y0), by1.max(y1)));
                        } else {
                            out.push(Highlight {
                                page: *page,
                                x: bx0,
                                y: by0,
                                width: bx1 - bx0,
                                height: by1 - by0,
                            });
                            band = Some((x0, x1, y0, y1));
                        }
                    }
                    None => band = Some((x0, x1, y0, y1)),
                }
            }
            if let Some((x0, x1, y0, y1)) = band {
                out.push(Highlight {
                    page: *page,
                    x: x0,
                    y: y0,
                    width: x1 - x0,
                    height: y1 - y0,
                });
            }
        }
        out
    }

    /// The selected text, pages in order, line breaks as single spaces —
    /// observed: a selection across lines reads with newlines in the DOM's
    /// `getSelection()` but copies as one space-joined line, so that is what
    /// the clipboard gets here too.
    pub fn text(&self, pages: &[(usize, &[PdfGlyph])]) -> String {
        let mut out = String::new();
        for (page, glyphs) in pages {
            let Some((first, last)) = self.range(*page, glyphs.len()) else {
                continue;
            };
            if !out.is_empty() {
                out.push(' ');
            }
            let run = &glyphs[first..=last];
            let mut previous: Option<&PdfGlyph> = None;
            for glyph in run {
                if let Some(last) = previous {
                    let size = last.height.max(glyph.height);
                    let moved_line = (glyph.y - last.y).abs() > size * 0.5;
                    let went_back = glyph.x < last.x - size;
                    if moved_line || went_back {
                        // A line break copies as one space, as Obsidian's does.
                        if !out.ends_with(' ') {
                            out.push(' ');
                        }
                    } else if glyph.x - (last.x + last.width) > size * 0.15 && !out.ends_with(' ') {
                        out.push(' ');
                    }
                }
                if glyph.text.chars().all(char::is_whitespace) {
                    if !out.ends_with(' ') && !out.is_empty() {
                        out.push(' ');
                    }
                } else {
                    out.push_str(&glyph.text);
                }
                previous = Some(glyph);
            }
        }
        out
    }
}

/// The glyph on `page` nearest a point in page points, for turning a pointer
/// into a [`Mark`]. Reads left-to-right within the line the point is on, and
/// **clamps**: a point past the end of a line picks the last glyph whose box
/// starts before it, so a drag to the margin still selects the line.
pub fn glyph_at(glyphs: &[PdfGlyph], x: f32, y: f32) -> usize {
    if glyphs.is_empty() {
        return 0;
    }
    // The line: glyphs whose band contains y, else the nearest band.
    let mut best_line: Vec<usize> = Vec::new();
    let mut best_distance = f32::INFINITY;
    for (ix, glyph) in glyphs.iter().enumerate() {
        let top = glyph.y;
        let bottom = glyph.y + glyph.height;
        let distance = if y < top {
            top - y
        } else if y > bottom {
            y - bottom
        } else {
            0.0
        };
        if distance < best_distance - 0.01 {
            best_distance = distance;
            best_line.clear();
            best_line.push(ix);
        } else if (distance - best_distance).abs() <= 0.01 {
            best_line.push(ix);
        }
    }
    best_line.sort_unstable();
    // Within the line: the glyph the x falls in, else the nearest to its left,
    // else the first.
    let mut chosen = best_line[0];
    for ix in best_line {
        let glyph = &glyphs[ix];
        if x >= glyph.x - glyph.width * 0.5 {
            chosen = ix;
        } else {
            break;
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(text: &str, x: f32, y: f32, width: f32) -> PdfGlyph {
        PdfGlyph {
            text: text.to_string(),
            x,
            y,
            width,
            height: 10.0,
        }
    }

    /// "Hi there" on one line, then "bye" on the next.
    fn page() -> Vec<PdfGlyph> {
        let mut glyphs = Vec::new();
        let mut x = 0.0;
        for c in "Hi".chars() {
            glyphs.push(glyph(&c.to_string(), x, 100.0, 5.0));
            x += 5.0;
        }
        x += 3.0;
        for c in "there".chars() {
            glyphs.push(glyph(&c.to_string(), x, 100.0, 5.0));
            x += 5.0;
        }
        let mut x = 0.0;
        for c in "bye".chars() {
            glyphs.push(glyph(&c.to_string(), x, 120.0, 5.0));
            x += 5.0;
        }
        glyphs
    }

    #[test]
    fn a_drag_either_way_selects_the_same_run() {
        let glyphs = page();
        let pages = [(0usize, glyphs.as_slice())];
        let forward = Selection::new(Mark { page: 0, glyph: 2 }, Mark { page: 0, glyph: 6 });
        let backward = Selection::new(Mark { page: 0, glyph: 6 }, Mark { page: 0, glyph: 2 });
        assert_eq!(forward, backward);
        assert_eq!(forward.text(&pages), "there");
    }

    #[test]
    fn a_run_on_one_line_is_one_box() {
        let glyphs = page();
        let pages = [(0usize, glyphs.as_slice())];
        let selection = Selection::new(Mark { page: 0, glyph: 0 }, Mark { page: 0, glyph: 6 });
        let boxes = selection.boxes(&pages, 1.0);
        assert_eq!(boxes.len(), 1, "{boxes:?}");
        assert!(close(boxes[0].x, 0.0));
        assert!(close(boxes[0].width, 38.0), "{:?}", boxes[0]);
    }

    #[test]
    fn a_run_across_lines_makes_one_box_per_line() {
        let glyphs = page();
        let pages = [(0usize, glyphs.as_slice())];
        let selection = Selection::new(Mark { page: 0, glyph: 0 }, Mark { page: 0, glyph: 9 });
        let boxes = selection.boxes(&pages, 1.0);
        assert_eq!(boxes.len(), 2, "{boxes:?}");
        assert!(close(boxes[0].y, 100.0));
        assert!(close(boxes[1].y, 120.0));
        assert_eq!(selection.text(&pages), "Hi there bye");
    }

    #[test]
    fn the_scale_multiplies_the_boxes_and_not_the_text() {
        let glyphs = page();
        let pages = [(0usize, glyphs.as_slice())];
        let selection = Selection::new(Mark { page: 0, glyph: 0 }, Mark { page: 0, glyph: 1 });
        let boxes = selection.boxes(&pages, 2.0);
        assert!(close(boxes[0].width, 20.0), "{:?}", boxes[0]);
        assert_eq!(selection.text(&pages), "Hi");
    }

    #[test]
    fn a_selection_across_pages_reads_down_the_pages() {
        let first = vec![glyph("a", 0.0, 0.0, 5.0)];
        let second = vec![glyph("b", 0.0, 0.0, 5.0)];
        let pages = [(0usize, first.as_slice()), (1usize, second.as_slice())];
        let selection = Selection::new(Mark { page: 0, glyph: 0 }, Mark { page: 1, glyph: 0 });
        assert_eq!(selection.text(&pages), "a b");
        assert_eq!(selection.boxes(&pages, 1.0).len(), 2);
    }

    #[test]
    fn an_empty_page_is_skipped() {
        let empty: Vec<PdfGlyph> = Vec::new();
        let glyphs = vec![glyph("a", 0.0, 0.0, 5.0)];
        let pages = [(0usize, empty.as_slice()), (1usize, glyphs.as_slice())];
        let selection = Selection::new(Mark { page: 0, glyph: 0 }, Mark { page: 1, glyph: 0 });
        assert_eq!(selection.text(&pages), "a");
    }

    #[test]
    fn glyph_at_picks_the_line_under_the_pointer() {
        let glyphs = page();
        // Inside "there" second line.
        assert_eq!(glyph_at(&glyphs, 20.0, 122.0), 9);
        // Past the end of the first line: the last glyph of it.
        assert_eq!(glyph_at(&glyphs, 500.0, 101.0), 6);
        // Between the lines: whichever is nearer; 110 is equidistant, the
        // earlier line wins by the `<` tie rule.
        assert_eq!(glyph_at(&glyphs, 0.0, 109.0), 0);
    }

    #[test]
    fn glyph_at_past_the_left_margin_is_the_first_glyph() {
        let glyphs = page();
        assert_eq!(glyph_at(&glyphs, -50.0, 100.0), 0);
    }

    #[test]
    fn glyph_at_on_no_glyphs_is_zero() {
        assert_eq!(glyph_at(&[], 1.0, 1.0), 0);
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }
}
