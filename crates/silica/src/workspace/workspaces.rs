//! Named workspace layouts: `.silica/workspaces/<name>.json` files in the
//! vault, each holding one session's open tab groups and side-dock tabs,
//! with commands to save a layout, put one back on screen, and switch
//! between them from a modal.

use super::*;

use std::time::SystemTime;

/// The vault folder the saved layouts live in.
pub(crate) const WORKSPACES_DIR: &str = ".silica/workspaces";

/// Every name a saved layout goes by, in alphabetical order.
pub(crate) fn list_workspaces(state: &VaultState) -> Result<Vec<String>, String> {
    let dir = state.root.join(WORKSPACES_DIR);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{} could not be read: {e}", dir.display())),
    };
    let mut names = Vec::new();
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_string_lossy().strip_suffix(".json") {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

impl Workspace {
    /// The layout the session is in right now: the main area's split tree
    /// and tab groups, the active tab, the mode and history, and the side
    /// docks' tabs and widths — everything the shell would show again when
    /// the layout is loaded.
    pub(crate) fn capture_layout(&self, name: &str) -> Value {
        let mut groups = Vec::new();
        for group in &self.groups {
            let mut tabs = Vec::new();
            for tab in &group.tabs {
                let mut history = Vec::new();
                for entry in &tab.history {
                    history.push(json!({
                        "path": entry.path,
                        "mode": entry.mode.as_str(),
                        "offset": entry.offset,
                    }));
                }
                tabs.push(json!({
                    "path": tab.path,
                    "mode": tab.mode.as_str(),
                    "history": history,
                    "history_ix": tab.history_ix,
                    "leaf": tab.leaf,
                    "view_state": tab.view_state,
                }));
            }
            groups.push(json!({
                "id": group.id,
                "tabs": tabs,
                "active_tab": group.active_tab,
                "stacked": group.stacked,
                "dimension": group.dimension(),
            }));
        }
        json!({
            "name": name,
            "created": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            "left_tab": left_tab_name(&self.left_tab),
            "right_tab": right_tab_name(&self.right_tab),
            "left_width": self.left_width,
            "right_width": self.right_width,
            "mode": self.mode.as_str(),
            "current": self.current,
            "history": self
                .history
                .iter()
                .map(|entry| {
                    json!({
                        "path": entry.path,
                        "mode": entry.mode.as_str(),
                        "offset": entry.offset,
                    })
                })
                .collect::<Vec<Value>>(),
            "history_ix": self.history_ix,
            "active_group": self.active_group_id(),
            "groups": groups,
            "layout_tree": serialize_layout(&self.layout_tree),
        })
    }

    /// Reads `.silica/workspaces/<name>.json` and puts its layout back on
    /// screen: every group rebuilt with its tabs, the split tree restored,
    /// and the side docks put back where they were.
    pub(crate) fn load_workspace(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let Some(state) = self.state.as_ref() else {
            return Err("no vault is open".to_string());
        };
        let path = format!("{WORKSPACES_DIR}/{name}.json");
        let text = std::fs::read_to_string(state.root.join(&path))
            .map_err(|_| format!("no workspace named {name}"))?;
        let layout: Value = serde_json::from_str(&text)
            .map_err(|e| format!("workspace {name} is not valid JSON: {e}"))?;
        self.apply_layout(&layout, window, cx)?;
        Ok(())
    }

    /// Puts a parsed layout document on screen, group by group. The active
    /// group is painted last, so it is what remains on screen.
    pub(crate) fn apply_layout(
        &mut self,
        layout: &Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let document = layout
            .as_object()
            .ok_or_else(|| "the workspace layout is not an object".to_string())?;

        let left_tab = document
            .get("left_tab")
            .and_then(Value::as_str)
            .map(left_tab_of)
            .unwrap_or(LeftTab::Files);
        let right_tab = document
            .get("right_tab")
            .and_then(Value::as_str)
            .map(right_tab_of)
            .unwrap_or(RightTab::Backlinks);
        let left_width = document
            .get("left_width")
            .and_then(Value::as_f64)
            .unwrap_or(300.0);
        let right_width = document
            .get("right_width")
            .and_then(Value::as_f64)
            .unwrap_or(300.0);

        // * Rebuild every saved group. `new_group` hands out fresh ids, so
        //   the saved id is stamped onto the group and its `tabs` node.
        let saved_groups = document
            .get("groups")
            .and_then(Value::as_array)
            .ok_or_else(|| "the workspace layout has no groups".to_string())?;
        let mut by_id: HashMap<u64, usize> = HashMap::new();
        for saved in saved_groups {
            let id = saved
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| "a group has no id".to_string())?;
            let mode = saved
                .get("mode")
                .and_then(|v| v.as_str())
                .and_then(ViewMode::parse)
                .unwrap_or(ViewMode::Reading);
            let _fresh = self.new_group(mode, window, cx);
            let group = self.groups.last_mut().expect("the group just made");
            group.id = id;
            group.node.insert("id".into(), json!(id));
            group.stacked = saved
                .get("stacked")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if let Some(dimension) = saved.get("dimension").and_then(Value::as_f64) {
                group.node.insert("dimension".into(), json!(dimension));
            }
            group.mode_before_graph = mode;
            let mut tabs = Vec::new();
            if let Some(saved_tabs) = saved.get("tabs").and_then(Value::as_array) {
                for saved_tab in saved_tabs {
                    let mut history = Vec::new();
                    if let Some(saved_history) = saved_tab.get("history").and_then(Value::as_array)
                    {
                        for entry in saved_history {
                            history.push(HistoryEntry {
                                path: entry
                                    .get("path")
                                    .and_then(Value::as_str)
                                    .ok_or_else(|| "a history entry has no path".to_string())?
                                    .to_string(),
                                mode: entry
                                    .get("mode")
                                    .and_then(|v| v.as_str())
                                    .and_then(ViewMode::parse)
                                    .unwrap_or(ViewMode::Reading),
                                offset: entry.get("offset").and_then(Value::as_u64).unwrap_or(0)
                                    as usize,
                            });
                        }
                    }
                    let history_ix = (saved_tab
                        .get("history_ix")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize)
                        .min(history.len());
                    tabs.push(Tab {
                        path: saved_tab
                            .get("path")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        mode: saved_tab
                            .get("mode")
                            .and_then(|v| v.as_str())
                            .and_then(ViewMode::parse)
                            .unwrap_or(ViewMode::Reading),
                        history,
                        history_ix,
                        leaf: saved_tab.get("leaf").cloned(),
                        view_state: saved_tab.get("view_state").cloned(),
                    });
                }
            }
            let active_tab = (saved.get("active_tab").and_then(Value::as_u64).unwrap_or(0)
                as usize)
                .min(tabs.len().saturating_sub(1));
            group.tabs = tabs;
            group.active_tab = active_tab;
            by_id.insert(id, self.groups.len() - 1);
        }

        // * Rebuild the split tree, refusing one that names a group the
        //   layout no longer has.
        let known: HashSet<u64> = by_id.keys().copied().collect();
        self.layout_tree = if let Some(saved_tree) = document.get("layout_tree") {
            parse_layout_tree(saved_tree, &known)?
        } else if let Some(&id) = by_id.keys().next() {
            Layout::Group(id)
        } else {
            let _fresh = self.new_group(ViewMode::Reading, window, cx);
            let group = self.groups.last_mut().expect("the group just made");
            let id = group.id;
            by_id.insert(id, self.groups.len() - 1);
            Layout::Group(id)
        };
        self.next_group_id = by_id.keys().max().copied().map(|id| id + 1).unwrap_or(1);

        let active_id = document.get("active_group").and_then(Value::as_u64);
        self.active_group = active_id
            .and_then(|id| by_id.get(&id))
            .copied()
            .unwrap_or(0);
        self.group_mru = active_id.into_iter().collect();

        self.left_tab = left_tab;
        self.right_tab = right_tab;
        self.left_width = left_width;
        self.right_width = right_width;
        self.mode = document
            .get("mode")
            .and_then(|v| v.as_str())
            .and_then(ViewMode::parse)
            .unwrap_or(ViewMode::Reading);
        self.current = document
            .get("current")
            .and_then(Value::as_str)
            .map(str::to_string);
        self.history = document
            .get("history")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        let path = entry.get("path").and_then(Value::as_str)?;
                        let mode = entry
                            .get("mode")
                            .and_then(|v| v.as_str())
                            .and_then(ViewMode::parse)?;
                        Some(HistoryEntry {
                            path: path.to_string(),
                            mode,
                            offset: entry.get("offset").and_then(Value::as_u64).unwrap_or(0)
                                as usize,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.history_ix = (document
            .get("history_ix")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize)
            .min(self.history.len());

        // * Paint every group's active tab into its views, the active
        //   group last so it is what is on screen when the command returns.
        let mut order: Vec<usize> = (0..self.groups.len()).collect();
        order.sort_by_key(|&ix| ix != self.active_group);
        for ix in order {
            self.load_group(ix, cx);
            if let Some(path) = self.current.as_deref() {
                let loaded = self
                    .state
                    .as_ref()
                    .map_or(false, |s| s.vault().absolute(path).is_file());
                if loaded {
                    self.load_tab(cx);
                }
            }
        }
        cx.notify();
        Ok(())
    }

    /// The workspaces modal: every saved layout as a row the user can
    /// switch to. Clicking outside does not close it; a row does.
    pub(crate) fn render_workspaces_modal(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(state) = self.state.as_ref() else {
            return div().into_any_element();
        };
        let Ok(names) = list_workspaces(state) else {
            return div().into_any_element();
        };

        let list = div()
            .id("workspaces-modal")
            .flex()
            .flex_col()
            .min_w(px(300.))
            .max_w(px(520.))
            .p(px(6.))
            .bg(theme.bg_secondary)
            .border_1()
            .border_color(theme.border)
            .rounded(px(8.))
            .shadow_lg()
            .text_size(px(FONT_UI_SMALL))
            .text_color(theme.text)
            .child(
                div()
                    .flex_1()
                    .text_color(theme.text_muted)
                    .child("Workspaces"),
            );
        let list = names.into_iter().fold(list, |list, name| {
            let name = name.clone();
            let click_name = name.clone();
            list.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .px(px(8.))
                    .h(px(25.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_alt))
                    .id(name.clone())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.close_modal(window, cx);
                        if let Err(reason) = this.load_workspace(&click_name, window, cx) {
                            this.report(reason, cx);
                        }
                    }))
                    .child(icon("layout-dashboard", ICON_SIZE, theme.text_muted))
                    .child(SharedString::from(name)),
            )
        });
        list.into_any_element()
    }
}

fn left_tab_of(name: &str) -> LeftTab {
    match name {
        "search" => LeftTab::Search,
        "bookmarks" => LeftTab::Bookmarks,
        _ => LeftTab::Files,
    }
}

fn right_tab_of(name: &str) -> RightTab {
    match name {
        "outgoing" => RightTab::Outgoing,
        "tags" => RightTab::Tags,
        "graph" => RightTab::Graph,
        "properties" => RightTab::Properties,
        "outline" => RightTab::Outline,
        _ => RightTab::Backlinks,
    }
}

fn left_tab_name(tab: &LeftTab) -> &'static str {
    match tab {
        LeftTab::Files => "files",
        LeftTab::Search => "search",
        LeftTab::Bookmarks => "bookmarks",
    }
}

fn right_tab_name(tab: &RightTab) -> &'static str {
    match tab {
        RightTab::Backlinks => "backlinks",
        RightTab::Outgoing => "outgoing",
        RightTab::Tags => "tags",
        RightTab::Graph => "graph",
        RightTab::Properties => "properties",
        RightTab::Outline => "outline",
    }
}

/// One node of the saved split tree, as `workspace.json`'s `main` records
/// it: a `group` leaf holding a group id, or a `split` with a direction,
/// children and the split's own keys (`id`, `dimension`, anything else).
fn parse_layout_tree(value: &Value, known: &HashSet<u64>) -> Result<Layout, String> {
    let r#type = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "a layout node has no type".to_string())?;
    match r#type {
        "group" => {
            let id = value
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| "a group node has no id".to_string())?;
            if !known.contains(&id) {
                return Err(format!("the layout refers to an unknown group {id}"));
            }
            Ok(Layout::Group(id))
        }
        "split" => {
            let direction = match value.get("direction").and_then(Value::as_str) {
                Some("horizontal") => SplitDirection::Horizontal,
                _ => SplitDirection::Vertical,
            };
            let children = value
                .get("children")
                .and_then(Value::as_array)
                .ok_or_else(|| "a split has no children".to_string())?;
            if children.is_empty() {
                return Err("a split has no children".to_string());
            }
            let mut built = Vec::new();
            for child in children {
                built.push(parse_layout_tree(child, known)?);
            }
            let mut node = Map::new();
            for (key, val) in value.as_object().expect("a split is an object") {
                if key != "type" && key != "direction" && key != "children" {
                    node.insert(key.clone(), val.clone());
                }
            }
            Ok(Layout::Split {
                direction,
                children: built,
                node,
            })
        }
        other => Err(format!("unknown layout node type '{other}'")),
    }
}

/// The split tree back into the `workspace.json` shape the shell writes:
/// `group` leaves and `split` nodes with their own keys kept.
fn serialize_layout(node: &Layout) -> Value {
    match node {
        Layout::Group(id) => json!({ "type": "group", "id": id }),
        Layout::Split {
            direction,
            children,
            node,
        } => {
            let mut object = node.clone();
            object.insert("type".into(), json!("split"));
            object.insert("direction".into(), json!(direction.as_str()));
            object.insert(
                "children".into(),
                json!(children
                    .iter()
                    .map(serialize_layout)
                    .collect::<Vec<Value>>()),
            );
            Value::Object(object)
        }
    }
}
