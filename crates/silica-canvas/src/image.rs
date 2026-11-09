//! The canvas exported as a PNG image.
//! The picture is drawn as SVG rather than captured because gpui's
//! `render_to_image` is a test-only entry point.
//! Nodes are drawn as cards with their title or file name, edges as
//! straight lines between them. The chrome (selection handles, scrollbars,
//! the toolbar) is not part of the picture.

use silica_formats::canvas::{Edge, EndStyle};

/// The colours the canvas paints with, as `#rrggbb`.
pub struct Palette {
    pub background: String,
    pub node_fill: String,
    pub node_stroke: String,
    pub node_text: String,
    pub edge: String,
}

/// One node positioned for export, already translated into the SVG canvas.
pub struct Placed {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Draw the canvas as an SVG document — `width` by `height` units.
pub fn svg(width: u32, height: u32, nodes: &[Placed], edges: &[Edge], palette: &Palette) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    ));
    out.push_str(&format!(
        "<rect width=\"{width}\" height=\"{height}\" fill=\"{}\" />",
        palette.background
    ));
    // Arrowhead marker
    out.push_str(
        "<defs><marker id=\"a\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"",
    );
    out.push_str(&palette.edge);
    out.push_str("\" /></marker></defs>");
    // Edges first (behind nodes)
    let has_arrow = edges.iter().any(|e| e.to_end == Some(EndStyle::Arrow));
    out.push_str(&format!(
        "<g stroke=\"{}\" stroke-width=\"2\" fill=\"none\"{}>",
        palette.edge,
        if has_arrow {
            " marker-end=\"url(#a)\""
        } else {
            ""
        }
    ));
    for edge in edges {
        let a = nodes.iter().find(|n| n.node_id == edge.from_node);
        let b = nodes.iter().find(|n| n.node_id == edge.to_node);
        if let (Some(ai), Some(bi)) = (a, b) {
            let dx = bi.x + bi.width / 2.0 - ai.x - ai.width / 2.0;
            let dy = bi.y + bi.height / 2.0 - ai.y - ai.height / 2.0;
            let len = (dx * dx + dy * dy).sqrt();
            if len > f64::EPSILON {
                let ux = dx / len;
                let uy = dy / len;
                let ra = ai.width / 2.0;
                let rb = bi.width / 2.0;
                out.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" />",
                    ai.x + ux * ra,
                    ai.y + uy * ra,
                    bi.x - ux * rb,
                    bi.y - uy * rb
                ));
            }
        }
    }
    out.push_str("</g>");
    // Nodes
    out.push_str(&format!(
        "<g stroke=\"{}\" stroke-width=\"2\" fill=\"{}\">",
        palette.node_stroke, palette.node_fill
    ));
    for p in nodes {
        if p.width <= 0.0 || p.height <= 0.0 {
            continue;
        }
        let rx = 8.0;
        out.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\" />",
            p.x, p.y, p.width, p.height, rx, palette.node_fill, palette.node_stroke
        ));
        if !p.node_id.is_empty() {
            let label = escape(&p.node_id);
            out.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" font-size=\"11\" font-family=\"DejaVu Sans, Noto Sans, sans-serif\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                p.x + p.width / 2.0,
                p.y + p.height / 2.0 + 4.0,
                palette.node_text,
                label
            ));
        }
    }
    out.push_str("</g>");
    out.push_str("</svg>");
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

/// Escape text for safe inclusion in an SVG text node.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, x: f64, y: f64) -> Placed {
        Placed {
            node_id: id.into(),
            x,
            y,
            width: 100.0,
            height: 60.0,
        }
    }

    fn palette() -> Palette {
        Palette {
            background: "#1e1e1e".into(),
            node_fill: "#2d2d2d".into(),
            node_stroke: "#5a5a5a".into(),
            node_text: "#e0e0e0".into(),
            edge: "#888888".into(),
        }
    }

    #[test]
    fn every_node_is_a_rectangle_with_a_label() {
        let nodes = vec![node("a", 20.0, 20.0), node("b", 80.0, 20.0)];
        let doc = svg(200, 100, &nodes, &[], &palette());
        assert_eq!(doc.matches("<rect").count(), 3);
        assert!(doc.contains(">a<") && doc.contains(">b<"));
        assert_eq!(doc.matches("<line").count(), 0);
    }

    #[test]
    fn edges_between_nodes_are_lines() {
        let nodes = vec![node("a", 20.0, 20.0), node("b", 80.0, 20.0)];
        let edges = vec![Edge::new("e1", "a", "b")];
        let doc = svg(200, 100, &nodes, &edges, &palette());
        assert_eq!(doc.matches("<line").count(), 1);
    }

    #[test]
    fn a_label_with_markup_is_escaped() {
        let nodes = vec![Placed {
            node_id: "a<b>&c".into(),
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 60.0,
        }];
        let doc = svg(30, 30, &nodes, &[], &palette());
        assert!(doc.contains("a&lt;b&gt;&amp;c"), "{doc}");
    }

    #[test]
    fn the_raster_is_a_png() {
        let nodes = vec![node("a", 10.0, 10.0)];
        let doc = svg(30, 30, &nodes, &[], &palette());
        let bytes = png(&doc, 30, 30).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (60, 60));
    }
}
