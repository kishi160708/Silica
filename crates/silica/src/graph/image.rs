//! The graph as a PNG.
//!
//! Obsidian's graph has no export, so nothing here copies a menu: the file
//! is written from the same node positions and radii the view paints, at a
//! size and scale the caller picks, so what a reader saves is what they see.
//! It is drawn rather than captured because gpui's `render_to_image` is a
//! test-only entry point.
//!
//! Only what the view draws is drawn: the nodes, their labels, the edges and
//! their arrowheads. The control panel, the tooltip and the hover highlight
//! are chrome around the picture, not part of it.

use crate::graph::view::Node;
use image::RgbaImage;

/// One node's position and radius, in world units; the caller scales.
pub struct Placed {
    /// Index into the node list handed to [`render`].
    pub node: usize,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

/// The colours the view paints with, as `#rrggbb`.
pub struct Palette {
    pub background: String,
    pub node: String,
    pub edge: String,
    pub label: String,
}

/// Draw the graph as an SVG document — `width` by `height` units, positions
/// already translated into it.
///
/// The picture goes through SVG rather than pixels because the labels are
/// text: shaping them is the font stack's job, and the same rasteriser that
/// draws Mermaid can turn this back into a PNG. Nodes, their labels, the
/// edges and their arrowheads are what the view draws; the panel, tooltip
/// and hover highlight are chrome around the picture, not part of it.
pub fn svg(
    width: u32,
    height: u32,
    nodes: &[Node],
    placed: &[Placed],
    edges: &[(u32, u32)],
    arrows: bool,
    palette: &Palette,
) -> String {
    let mut x = vec![0.0f32; nodes.len()];
    let mut y = vec![0.0f32; nodes.len()];
    let mut r = vec![0.0f32; nodes.len()];
    for p in placed {
        if p.node < nodes.len() {
            x[p.node] = p.x;
            y[p.node] = p.y;
            r[p.node] = p.radius;
        }
    }
    let mut out = String::with_capacity(4096);
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    ));
    out.push_str(&format!(
        "<rect width=\"{width}\" height=\"{height}\" fill=\"{}\"/>",
        palette.background
    ));
    out.push_str("<defs><marker id=\"a\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"");
    out.push_str(&palette.edge);
    out.push_str("\"/></marker></defs>");

    out.push_str(&format!(
        "<g stroke=\"{}\" stroke-width=\"1\" fill=\"none\"{}>",
        palette.edge,
        if arrows {
            " marker-end=\"url(#a)\""
        } else {
            ""
        }
    ));
    for &(a, b) in edges {
        let (a, b) = (a as usize, b as usize);
        if a >= nodes.len() || b >= nodes.len() {
            continue;
        }
        // Trim to the rims so a head does not sit inside its own disc.
        let (dx, dy) = (x[b] - x[a], y[b] - y[a]);
        let len = (dx * dx + dy * dy).sqrt();
        if len <= f32::EPSILON {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        out.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\"/>",
            x[a] + ux * r[a],
            y[a] + uy * r[a],
            x[b] - ux * r[b],
            y[b] - uy * r[b]
        ));
    }
    out.push_str("</g>");

    for i in 0..nodes.len() {
        if r[i] <= 0.0 {
            continue;
        }
        let fill = match nodes[i].color {
            Some(c) => hsla_hex(c),
            None => palette.node.clone(),
        };
        out.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{fill}\"/>",
            x[i], y[i], r[i]
        ));
    }

    out.push_str(&format!(
        // A generic `sans-serif` resolves to nothing in usvg, so the label
        // carries a family the font database actually has.
        "<g font-size=\"11\" font-family=\"DejaVu Sans, Noto Sans, sans-serif\" fill=\"{}\" text-anchor=\"middle\">",
        palette.label
    ));
    for i in 0..nodes.len() {
        if r[i] <= 0.0 || nodes[i].label.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\">{}</text>",
            x[i],
            y[i] + r[i] + 12.0,
            escape(&nodes[i].label)
        ));
    }
    out.push_str("</g></svg>");
    out
}

/// Rasterise an SVG document to PNG bytes, twice size for a sharp print.
pub fn png(svg: &str, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| e.to_string())?;
    let scale = 2.0f32;
    let (w, h) = (
        (width as f32 * scale).ceil().max(1.0) as u32,
        (height as f32 * scale).ceil().max(1.0) as u32,
    );
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or_else(|| "no pixmap".to_string())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| e.to_string())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn hsla_hex(color: gpui::Hsla) -> String {
    let rgba: gpui::Rgba = color.into();
    format!(
        "#{:02x}{:02x}{:02x}",
        (rgba.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba.b.clamp(0.0, 1.0) * 255.0).round() as u8
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::view::{Node, NodeKind};

    fn node(label: &str) -> Node {
        Node {
            id: label.into(),
            label: label.into(),
            kind: NodeKind::Note,
            degree: 0,
            color: None,
        }
    }

    fn palette() -> Palette {
        Palette {
            background: "#000000".into(),
            node: "#c8c8c8".into(),
            edge: "#5a5a5a".into(),
            label: "#eeeeee".into(),
        }
    }

    #[test]
    fn every_node_is_a_circle_with_a_label() {
        let nodes = vec![node("one"), node("two")];
        let placed = vec![
            Placed {
                node: 0,
                x: 20.0,
                y: 20.0,
                radius: 5.0,
            },
            Placed {
                node: 1,
                x: 80.0,
                y: 20.0,
                radius: 5.0,
            },
        ];
        let doc = svg(100, 40, &nodes, &placed, &[(0, 1)], false, &palette());
        assert_eq!(doc.matches("<circle").count(), 2);
        assert!(doc.contains(">one<") && doc.contains(">two<"));
        assert_eq!(doc.matches("<line").count(), 1);
    }

    #[test]
    fn the_edge_stops_at_both_rims() {
        let nodes = vec![node("a"), node("b")];
        let placed = vec![
            Placed {
                node: 0,
                x: 10.0,
                y: 10.0,
                radius: 4.0,
            },
            Placed {
                node: 1,
                x: 50.0,
                y: 10.0,
                radius: 4.0,
            },
        ];
        let doc = svg(60, 20, &nodes, &placed, &[(0, 1)], false, &palette());
        // From x=14 to x=46 at y=10.
        assert!(doc.contains("x1=\"14.00\""), "{doc}");
        assert!(doc.contains("x2=\"46.00\""), "{doc}");
    }

    #[test]
    fn arrows_add_the_marker() {
        let nodes = vec![node("a"), node("b")];
        let placed = vec![
            Placed {
                node: 0,
                x: 10.0,
                y: 10.0,
                radius: 4.0,
            },
            Placed {
                node: 1,
                x: 50.0,
                y: 10.0,
                radius: 4.0,
            },
        ];
        let plain = svg(60, 20, &nodes, &placed, &[(0, 1)], false, &palette());
        let arrowed = svg(60, 20, &nodes, &placed, &[(0, 1)], true, &palette());
        assert!(!plain.contains("marker-end"));
        assert!(arrowed.contains("marker-end"));
    }

    #[test]
    fn a_label_with_markup_is_escaped() {
        let nodes = vec![node("a<b>&c")];
        let placed = vec![Placed {
            node: 0,
            x: 10.0,
            y: 10.0,
            radius: 4.0,
        }];
        let doc = svg(30, 30, &nodes, &placed, &[], false, &palette());
        assert!(doc.contains("a&lt;b&gt;&amp;c"), "{doc}");
    }

    #[test]
    fn a_degenerate_edge_is_skipped() {
        let nodes = vec![node("a"), node("b")];
        let placed = vec![
            Placed {
                node: 0,
                x: 10.0,
                y: 10.0,
                radius: 4.0,
            },
            Placed {
                node: 1,
                x: 10.0,
                y: 10.0,
                radius: 4.0,
            },
        ];
        let doc = svg(30, 30, &nodes, &placed, &[(0, 1)], false, &palette());
        assert_eq!(doc.matches("<line").count(), 0);
    }

    #[test]
    fn labels_are_ink_in_the_raster() {
        let nodes = vec![node("Rocks")];
        let placed = vec![Placed {
            node: 0,
            x: 60.0,
            y: 30.0,
            radius: 4.0,
        }];
        let doc = svg(120, 60, &nodes, &placed, &[], false, &palette());
        let bytes = png(&doc, 120, 60).unwrap();
        let image = image::load_from_memory(&bytes).unwrap().to_rgba8();
        // The label sits under the node; count its bright pixels there.
        let label = (0..60u32)
            .flat_map(|x| (34..52u32).map(move |y| (x, y)))
            .filter(|(x, y)| image.get_pixel(x * 2, y * 2)[0] > 150)
            .count();
        assert!(label > 20, "label did not rasterise ({label} lit pixels)");
    }

    #[test]
    fn the_raster_is_a_png() {
        let nodes = vec![node("a")];
        let placed = vec![Placed {
            node: 0,
            x: 10.0,
            y: 10.0,
            radius: 4.0,
        }];
        let doc = svg(30, 30, &nodes, &placed, &[], false, &palette());
        let bytes = png(&doc, 30, 30).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        // Twice size, as the metadata records.
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (60, 60));
    }
}
