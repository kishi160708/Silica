//! Bases (`.base`) YAML files.
//!
//! Top-level keys per the published syntax: `filters`, `formulas`, `properties`,
//! `summaries`, `views`. Statement strings are kept verbatim; the expression
//! language gets its own parser in a later milestone.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Base {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Filter>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub formulas: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, PropertyConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub summaries: BTreeMap<String, String>,
    #[serde(default)]
    pub views: Vec<View>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// A filter is a statement string or a recursive `and` / `or` / `not` list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    Statement(String),
    And { and: Vec<Filter> },
    Or { or: Vec<Filter> },
    Not { not: Vec<Filter> },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyConfig {
    #[serde(
        default,
        rename = "displayName",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_name: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupBy {
    pub property: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct View {
    /// `table`, `cards`, `list`, `kanban`, `map`, or a plugin type.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Filter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
    #[serde(default, rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<GroupBy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub summaries: BTreeMap<String, String>,
    /// Layout-specific state (sort, column widths, card size, ...) whose exact
    /// keys Obsidian does not publish; preserved verbatim.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl Base {
    pub fn from_yaml(text: &str) -> Result<Base, serde_yaml::Error> {
        serde_yaml::from_str(text)
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
filters:
  or:
    - file.hasTag("tag")
    - and:
        - file.hasTag("book")
        - file.hasLink("Textbook")
    - not:
        - file.inFolder("Required Reading")
formulas:
  ppu: "(price / age).toFixed(2)"
properties:
  status:
    displayName: Status
  formula.ppu:
    displayName: Price per unit
summaries:
  customAverage: 'values.mean().round(3)'
views:
  - type: table
    name: "My table"
    limit: 10
    groupBy:
      property: note.age
      direction: DESC
    filters:
      and:
        - 'status != "done"'
    order:
      - file.name
      - formula.ppu
    summaries:
      formula.ppu: Average
    columnSize:
      file.name: 240
"#;

    #[test]
    fn parses_published_example() {
        let base = Base::from_yaml(SAMPLE).unwrap();
        match base.filters.as_ref().unwrap() {
            Filter::Or { or } => assert_eq!(or.len(), 3),
            other => panic!("unexpected top filter {other:?}"),
        }
        assert_eq!(base.formulas["ppu"], "(price / age).toFixed(2)");
        assert_eq!(
            base.properties["formula.ppu"].display_name.as_deref(),
            Some("Price per unit")
        );
        let view = &base.views[0];
        assert_eq!(view.kind, "table");
        assert_eq!(view.limit, Some(10));
        assert_eq!(view.group_by.as_ref().unwrap().property, "note.age");
        assert_eq!(view.order, vec!["file.name", "formula.ppu"]);
        assert!(view.extra.contains_key("columnSize"));
    }

    #[test]
    fn round_trips_without_losing_keys() {
        let base = Base::from_yaml(SAMPLE).unwrap();
        let text = base.to_yaml().unwrap();
        assert_eq!(Base::from_yaml(&text).unwrap(), base);
    }
}
