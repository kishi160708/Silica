//! JSON Canvas 1.0 (`.canvas`), as written by Obsidian.
//!
//! Nodes are kept in file order because order is z-order (earlier beneath later).
//! Unknown fields are preserved for plugin data.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Canvas {
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Text,
    File,
    Link,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundStyle {
    Cover,
    Ratio,
    Repeat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: NodeType,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    /// `"1"`..`"6"` preset or `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    // text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    // file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpath: Option<String>,
    // link
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    // group
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(
        default,
        rename = "backgroundStyle",
        skip_serializing_if = "Option::is_none"
    )]
    pub background_style: Option<BackgroundStyle>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndStyle {
    None,
    Arrow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    #[serde(rename = "fromNode")]
    pub from_node: String,
    #[serde(default, rename = "fromSide", skip_serializing_if = "Option::is_none")]
    pub from_side: Option<Side>,
    /// Defaults to `none`.
    #[serde(default, rename = "fromEnd", skip_serializing_if = "Option::is_none")]
    pub from_end: Option<EndStyle>,
    #[serde(rename = "toNode")]
    pub to_node: String,
    #[serde(default, rename = "toSide", skip_serializing_if = "Option::is_none")]
    pub to_side: Option<Side>,
    /// Defaults to `arrow`.
    #[serde(default, rename = "toEnd", skip_serializing_if = "Option::is_none")]
    pub to_end: Option<EndStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Canvas {
    pub fn from_json(text: &str) -> serde_json::Result<Canvas> {
        serde_json::from_str(text)
    }

    /// Obsidian writes canvases with tab indentation.
    pub fn to_json(&self) -> serde_json::Result<String> {
        let mut buf = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(b"\t");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
        self.serialize(&mut ser)?;
        Ok(String::from_utf8(buf).expect("serde_json emits UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"{
	"nodes":[
		{"id":"a1","type":"text","x":0,"y":0,"width":250,"height":60,"text":"Hello","color":"1"},
		{"id":"b2","type":"file","x":300,"y":0,"width":400,"height":400,"file":"Notes/Plan.md","subpath":"#Goals"},
		{"id":"c3","type":"link","x":0,"y":200,"width":300,"height":200,"url":"https://example.com"},
		{"id":"g4","type":"group","x":-20,"y":-20,"width":800,"height":700,"label":"Area","backgroundStyle":"cover","pluginData":{"k":1}}
	],
	"edges":[
		{"id":"e1","fromNode":"a1","fromSide":"right","toNode":"b2","toSide":"left","toEnd":"arrow","label":"leads to"}
	]
}"##;

    #[test]
    fn parses_all_node_types_and_keeps_unknown_fields() {
        let canvas = Canvas::from_json(SAMPLE).unwrap();
        assert_eq!(canvas.nodes.len(), 4);
        assert_eq!(canvas.nodes[0].kind, NodeType::Text);
        assert_eq!(canvas.nodes[1].subpath.as_deref(), Some("#Goals"));
        assert_eq!(
            canvas.nodes[3].background_style,
            Some(BackgroundStyle::Cover)
        );
        assert_eq!(canvas.nodes[3].extra["pluginData"]["k"], 1);
        assert_eq!(canvas.edges[0].to_end, Some(EndStyle::Arrow));
    }

    #[test]
    fn round_trips_through_json() {
        let canvas = Canvas::from_json(SAMPLE).unwrap();
        let text = canvas.to_json().unwrap();
        assert!(text.starts_with("{\n\t\"nodes\""));
        assert_eq!(Canvas::from_json(&text).unwrap(), canvas);
    }
}
