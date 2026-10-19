//! The graph element: builds nodes and edges from the vault cache, runs the
//! force layout on the background executor, and paints through a canvas
//! with zoom, pan, drag, hover and click.
//!
//! Filters, groups, display and forces come from `graph.json`; the global
//! graph shows every note, the local graph the current note and its
//! neighbors.

use super::layout::{Forces, Layout};
use crate::search::inputs;
use crate::theme::Theme;
use crate::workspace::VaultState;
use gpui::{
    actions, canvas, div, fill, point, prelude::*, px, size, App, Bounds, Context, DispatchPhase,
    EventEmitter, FocusHandle, Focusable, Hsla, KeyBinding, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, Point, ScrollWheelEvent, SharedString,
    TextAlign, Window,
};
use _formats::config::GraphConfig;
use _formats::FileKind;
use _search::{Search, SearchFile};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

actions!(graph, [ZoomIn, ZoomOut, ResetView]);

pub fn bind_keys(cx: &mut App) {
    let ctx = Some("GraphView");
    cx.bind_keys([
        KeyBinding::new("ctrl-=", ZoomIn, ctx),
        KeyBinding::new("ctrl--", ZoomOut, ctx),
        KeyBinding::new("ctrl-0", ResetView, ctx),
    ]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Note,
    Attachment,
    Unresolved,
    Tag,
}

#[derive(Debug, Clone)]
pub struct Node {
    /// Vault path, link text for unresolved targets, `#tag` for tags.
    pub id: String,
    pub label: SharedString,
    pub kind: NodeKind,
    pub degree: usize,
    pub color: Option<Hsla>,
}

/// Nodes and edges ready for layout, built off the UI thread.
struct Built {
    nodes: Vec<Node>,
    edges: Vec<(u32, u32)>,
    neighbors: Vec<Vec<usize>>,
}

pub enum GraphEvent {
    Open(String),
}

struct Drag {
    node: Option<usize>,
    start: Point<Pixels>,
    last: Point<Pixels>,
    moved: bool,
}

pub struct GraphView {
    focus_handle: FocusHandle,
    state: Option<Arc<VaultState>>,
    config: GraphConfig,
    nodes: Arc<Vec<Node>>,
    edges: Arc<Vec<(u32, u32)>>,
    neighbors: Arc<Vec<Vec<usize>>>,
    index: HashMap<String, usize>,
    layout: Arc<Mutex<Layout>>,
    running: bool,
    scale: f32,
    /// Pan in screen pixels from the element center.
    offset: Point<f32>,
    bounds: Bounds<Pixels>,
    hovered: Option<usize>,
    drag: Option<Drag>,
    /// Local graph centered on this path, else the global graph.
    local: Option<String>,
    current: Option<String>,
    generation: u64,
    building: bool,
    /// The user zoomed or panned; stop auto-fitting the view.
    user_adjusted: bool,
}

impl EventEmitter<GraphEvent> for GraphView {}

impl Focusable for GraphView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl GraphView {
    pub fn new(cx: &mut Context<Self>) -> GraphView {
        GraphView {
            focus_handle: cx.focus_handle(),
            state: None,
            config: GraphConfig::default(),
            nodes: Arc::new(Vec::new()),
            edges: Arc::new(Vec::new()),
            neighbors: Arc::new(Vec::new()),
            index: HashMap::new(),
            layout: Arc::new(Mutex::new(Layout::new(0, Vec::new(), Forces::default()))),
            running: false,
            scale: 1.0,
            offset: point(0.0, 0.0),
            bounds: Bounds::default(),
            hovered: None,
            drag: None,
            local: None,
            current: None,
            generation: 0,
            building: false,
            user_adjusted: false,
        }
    }

    pub fn set_state(&mut self, state: Arc<VaultState>, cx: &mut Context<Self>) {
        self.config = state.vault().graph_config().unwrap_or_default();
        if let Some(scale) = self.config.scale {
            self.scale = scale.clamp(0.1, 8.0);
        }
        self.state = Some(state);
        self.rebuild(cx);
    }

    /// The note to highlight, and to center a local graph on.
    pub fn set_current(&mut self, path: Option<String>, cx: &mut Context<Self>) {
        let changed = self.current != path;
        self.current = path.clone();
        if self.local.is_some() && changed {
            self.local = path;
            self.rebuild(cx);
        } else if changed {
            cx.notify();
        }
    }

    pub fn set_local(&mut self, local: bool, cx: &mut Context<Self>) {
        let want = if local { self.current.clone() } else { None };
        if self.local != want || (local && self.current.is_none()) {
            self.local = want;
            self.rebuild(cx);
        }
    }

    pub fn is_local(&self) -> bool {
        self.local.is_some()
    }

    /// What the graph is showing, for the control socket's state dump. The
    /// offset is the pan in screen pixels from the element centre, and
    /// `hovered` the label under the pointer, which is the only node the
    /// view names on screen.
    pub fn describe(&self) -> Value {
        json!({
            "local": self.is_local(),
            "nodes": self.nodes.len(),
            "scale": self.scale,
            "offset": [self.offset.x, self.offset.y],
            "hovered": self
                .hovered
                .and_then(|ix| self.nodes.get(ix))
                .map(|node| node.label.clone()),
        })
    }

    fn forces(&self) -> Forces {
        let d = Forces::default();
        Forces {
            center: self.config.center_strength.unwrap_or(d.center),
            repel: self.config.repel_strength.unwrap_or(d.repel),
            link: self.config.link_strength.unwrap_or(d.link),
            link_distance: self.config.link_distance.unwrap_or(d.link_distance),
        }
    }

    /// Rebuilds nodes and edges from the cache on the background executor,
    /// then restarts the simulation.
    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        self.generation += 1;
        let generation = self.generation;
        let config = self.config.clone();
        let local = self.local.clone();
        self.building = true;
        cx.spawn(async move |this, cx| {
            let built = cx
                .background_executor()
                .spawn(async move { build(&state, &config, local.as_deref()) })
                .await;
            this.update(cx, |view, cx| {
                if view.generation != generation {
                    return;
                }
                view.building = false;
                view.apply(built, cx);
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn apply(&mut self, built: Built, cx: &mut Context<Self>) {
        // Keep positions of nodes that survive the rebuild.
        let old: HashMap<String, (f32, f32)> = {
            let layout = self.layout.lock().unwrap_or_else(|e| e.into_inner());
            self.nodes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| layout.bodies.get(i).map(|b| (n.id.clone(), (b.x, b.y))))
                .collect()
        };
        let mut layout = Layout::new(built.nodes.len(), built.edges.clone(), self.forces());
        for (i, node) in built.nodes.iter().enumerate() {
            layout.bodies[i].mass = 1.0 + (node.degree as f32).sqrt() * 0.5;
            if let Some((x, y)) = old.get(&node.id) {
                layout.bodies[i].x = *x;
                layout.bodies[i].y = *y;
            }
        }
        self.index = built
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();
        self.nodes = Arc::new(built.nodes);
        self.edges = Arc::new(built.edges);
        self.neighbors = Arc::new(built.neighbors);
        self.hovered = None;
        self.layout = Arc::new(Mutex::new(layout));
        self.user_adjusted = false;
        self.start_simulation(cx);
        cx.notify();
    }

    /// Scales and centers the view on the whole layout.
    fn fit(&mut self) {
        let (x0, y0, x1, y1) = self
            .layout
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .bounds();
        let w = f32::from(self.bounds.size.width);
        let h = f32::from(self.bounds.size.height);
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let bw = (x1 - x0).max(1.0) + 120.0;
        let bh = (y1 - y0).max(1.0) + 120.0;
        self.scale = (w / bw).min(h / bh).clamp(0.05, 1.5);
        let cx = (x0 + x1) / 2.0;
        let cy = (y0 + y1) / 2.0;
        self.offset = point(-cx * self.scale, -cy * self.scale);
    }

    fn start_simulation(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        self.running = true;
        let layout = self.layout.clone();
        cx.spawn(async move |this, cx| {
            loop {
                let shared = layout.clone();
                let settled = cx
                    .background_executor()
                    .spawn(async move {
                        let mut l = shared.lock().unwrap_or_else(|e| e.into_inner());
                        for _ in 0..2 {
                            l.step();
                        }
                        l.is_settled()
                    })
                    .await;
                let alive = this
                    .update(cx, |view, cx| {
                        if !view.user_adjusted && view.drag.is_none() {
                            view.fit();
                        }
                        cx.notify();
                        // A rebuild swapped the layout: this loop is stale.
                        Arc::ptr_eq(&view.layout, &layout)
                    })
                    .unwrap_or(false);
                if !alive {
                    break;
                }
                if settled {
                    this.update(cx, |view, _| view.running = false).ok();
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
            }
        })
        .detach();
    }

    fn reheat(&mut self, cx: &mut Context<Self>) {
        self.layout
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .reheat();
        self.start_simulation(cx);
    }

    // ----- geometry --------------------------------------------------------------

    fn center(&self) -> Point<f32> {
        point(
            f32::from(self.bounds.origin.x + self.bounds.size.width / 2.) + self.offset.x,
            f32::from(self.bounds.origin.y + self.bounds.size.height / 2.) + self.offset.y,
        )
    }

    fn to_screen(&self, x: f32, y: f32) -> Point<Pixels> {
        let c = self.center();
        point(px(c.x + x * self.scale), px(c.y + y * self.scale))
    }

    fn to_world(&self, p: Point<Pixels>) -> (f32, f32) {
        let c = self.center();
        (
            (f32::from(p.x) - c.x) / self.scale,
            (f32::from(p.y) - c.y) / self.scale,
        )
    }

    fn node_radius(&self, node: &Node) -> f32 {
        let mult = self.config.node_size_multiplier.unwrap_or(1.0);
        (2.5 + (node.degree as f32).sqrt() * 1.2) * mult
    }

    /// Zoom level from which labels show: later for bigger graphs, as
    /// `textFadeMultiplier` shifts it.
    fn label_threshold(&self) -> f32 {
        let n = self.nodes.len().max(1) as f32;
        let fade = self.config.text_fade_multiplier.unwrap_or(0.0);
        0.4 + (n / 300.0).ln().max(0.0) * 0.5 + fade * 0.5
    }

    fn hit_test(&self, p: Point<Pixels>) -> Option<usize> {
        let layout = self.layout.lock().unwrap_or_else(|e| e.into_inner());
        let (wx, wy) = self.to_world(p);
        let mut best: Option<(usize, f32)> = None;
        for (i, b) in layout.bodies.iter().enumerate() {
            let r = (self.node_radius(&self.nodes[i]) + 4.0) / self.scale.max(0.01);
            let d2 = (b.x - wx).powi(2) + (b.y - wy).powi(2);
            if d2 <= r * r && best.is_none_or(|(_, bd)| d2 < bd) {
                best = Some((i, d2));
            }
        }
        best.map(|(i, _)| i)
    }

    fn zoom_at(&mut self, factor: f32, at: Point<Pixels>, cx: &mut Context<Self>) {
        self.user_adjusted = true;
        let (wx, wy) = self.to_world(at);
        self.scale = (self.scale * factor).clamp(0.05, 8.0);
        // Keep the world point under the cursor fixed.
        let c = self.center();
        let target = self.to_screen(wx, wy);
        self.offset.x += f32::from(at.x) - f32::from(target.x);
        self.offset.y += f32::from(at.y) - f32::from(target.y);
        let _ = c;
        cx.notify();
    }

    fn zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.bounds.center();
        self.zoom_at(1.25, at, cx);
    }

    fn zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.bounds.center();
        self.zoom_at(0.8, at, cx);
    }

    fn reset_view(&mut self, _: &ResetView, _: &mut Window, cx: &mut Context<Self>) {
        self.user_adjusted = false;
        self.fit();
        cx.notify();
    }

    // ----- mouse -----------------------------------------------------------------

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let node = self.hit_test(event.position);
        if let Some(ix) = node {
            if let Some(b) = self
                .layout
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .bodies
                .get_mut(ix)
            {
                b.pinned = true;
            }
        }
        self.drag = Some(Drag {
            node,
            start: event.position,
            last: event.position,
            moved: false,
        });
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if let Some(drag) = &mut self.drag {
            let dx = event.position.x - drag.last.x;
            let dy = event.position.y - drag.last.y;
            drag.last = event.position;
            let far = (event.position.x - drag.start.x).abs() > px(3.)
                || (event.position.y - drag.start.y).abs() > px(3.);
            if far {
                drag.moved = true;
            }
            match drag.node {
                Some(ix) => {
                    let (wx, wy) = self.to_world(event.position);
                    if let Some(b) = self
                        .layout
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .bodies
                        .get_mut(ix)
                    {
                        b.x = wx;
                        b.y = wy;
                    }
                    self.reheat(cx);
                }
                None => {
                    self.user_adjusted = true;
                    self.offset.x += f32::from(dx);
                    self.offset.y += f32::from(dy);
                }
            }
            cx.notify();
        } else {
            let hit = self.hit_test(event.position);
            if hit != self.hovered {
                self.hovered = hit;
                cx.notify();
            }
        }
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        if let Some(ix) = drag.node {
            if let Some(b) = self
                .layout
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .bodies
                .get_mut(ix)
            {
                b.pinned = false;
            }
            if !drag.moved && event.button == MouseButton::Left {
                if let Some(node) = self.nodes.get(ix) {
                    if matches!(node.kind, NodeKind::Note | NodeKind::Attachment) {
                        cx.emit(GraphEvent::Open(node.id.clone()));
                    }
                }
            }
        }
        cx.notify();
    }

    fn scroll(&mut self, event: &ScrollWheelEvent, line_height: Pixels, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(line_height);
        let factor = (-f32::from(delta.y) * 0.004).exp();
        self.zoom_at(factor, event.position, cx);
    }

    fn toggle_local(&mut self, cx: &mut Context<Self>) {
        let local = !self.is_local();
        self.set_local(local, cx);
    }

    fn toggle_orphans(&mut self, cx: &mut Context<Self>) {
        let now = !self.config.show_orphans.unwrap_or(true);
        self.config.show_orphans = Some(now);
        self.rebuild(cx);
    }

    fn toggle_attachments(&mut self, cx: &mut Context<Self>) {
        let now = !self.config.show_attachments.unwrap_or(false);
        self.config.show_attachments = Some(now);
        self.rebuild(cx);
    }

    fn toggle_tags(&mut self, cx: &mut Context<Self>) {
        let now = !self.config.show_tags.unwrap_or(false);
        self.config.show_tags = Some(now);
        self.rebuild(cx);
    }
}

/// Builds the node and edge lists for the current filters.
fn build(state: &VaultState, config: &GraphConfig, local: Option<&str>) -> Built {
    let cache = state.cache();
    let vault = state.vault();
    let texts = state.texts();
    let show_attachments = config.show_attachments.unwrap_or(false);
    let show_tags = config.show_tags.unwrap_or(false);
    let show_orphans = config.show_orphans.unwrap_or(true);
    let hide_unresolved = config.hide_unresolved.unwrap_or(false);

    let mut nodes: Vec<Node> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    fn add(
        index: &mut HashMap<String, usize>,
        nodes: &mut Vec<Node>,
        id: String,
        label: String,
        kind: NodeKind,
    ) -> usize {
        if let Some(&ix) = index.get(&id) {
            return ix;
        }
        let ix = nodes.len();
        index.insert(id.clone(), ix);
        nodes.push(Node {
            id,
            label: label.into(),
            kind,
            degree: 0,
            color: None,
        });
        ix
    }
    let label_of = |path: &str| -> String {
        let name = path.rsplit('/').next().unwrap_or(path);
        if FileKind::from_path(path) == FileKind::Markdown {
            name.trim_end_matches(".md").to_string()
        } else {
            name.to_string()
        }
    };

    // The search filter from graph.json narrows the note set.
    let filter = config
        .search
        .as_deref()
        .filter(|q| !q.trim().is_empty())
        .and_then(|q| Search::new(q).ok());
    let passes = |path: &str| -> bool {
        let Some(search) = &filter else { return true };
        let (Some(meta), Some(text)) = (cache.file(path), texts.get(path)) else {
            return true;
        };
        let (tags, blocks, sections, tasks) = inputs(meta, text);
        search
            .matches(&SearchFile {
                path,
                content: text,
                tags: &tags,
                frontmatter: meta.frontmatter.as_ref(),
                blocks: &blocks,
                sections: &sections,
                tasks: &tasks,
            })
            .is_some()
    };

    for (path, _) in cache.files() {
        if passes(path) {
            add(
                &mut index,
                &mut nodes,
                path.to_string(),
                label_of(path),
                NodeKind::Note,
            );
        }
    }
    if show_attachments {
        for file in vault.files() {
            if file.kind != FileKind::Markdown && file.kind != FileKind::Other {
                add(
                    &mut index,
                    &mut nodes,
                    file.path.clone(),
                    label_of(&file.path),
                    NodeKind::Attachment,
                );
            }
        }
    }

    let mut edge_set: HashSet<(u32, u32)> = HashSet::new();
    let note_paths: Vec<String> = cache.files().map(|(p, _)| p.to_string()).collect();
    for path in &note_paths {
        let Some(&from) = index.get(path) else {
            continue;
        };
        if let Some(targets) = cache.resolved_links(path) {
            for target in targets.keys() {
                let kind = FileKind::from_path(target);
                if kind != FileKind::Markdown && !show_attachments {
                    continue;
                }
                let to = match index.get(target) {
                    Some(&ix) => ix,
                    None if kind == FileKind::Markdown => continue, // filtered out
                    None => add(
                        &mut index,
                        &mut nodes,
                        target.clone(),
                        label_of(target),
                        NodeKind::Attachment,
                    ),
                };
                if from != to {
                    edge_set.insert((from as u32, to as u32));
                }
            }
        }
        if !hide_unresolved {
            if let Some(targets) = cache.unresolved_links(path) {
                for target in targets.keys() {
                    if target.is_empty() {
                        continue;
                    }
                    let to = add(
                        &mut index,
                        &mut nodes,
                        format!("unresolved:{}", target.to_lowercase()),
                        target.clone(),
                        NodeKind::Unresolved,
                    );
                    edge_set.insert((from as u32, to as u32));
                }
            }
        }
        if show_tags {
            if let Some(meta) = cache.file(path) {
                for tag in _cache::all_tags(meta) {
                    let to = add(
                        &mut index,
                        &mut nodes,
                        format!("tag:{}", tag.to_lowercase()),
                        format!("#{tag}"),
                        NodeKind::Tag,
                    );
                    edge_set.insert((from as u32, to as u32));
                }
            }
        }
    }

    // Color groups: first matching query wins.
    let groups: Vec<(Search, Hsla)> = config
        .color_groups
        .iter()
        .filter_map(|g| {
            let search = Search::new(&g.query).ok()?;
            let rgb = g.color.rgb;
            let mut color: Hsla = gpui::rgb(rgb).into();
            color.a = g.color.a.clamp(0.0, 1.0);
            Some((search, color))
        })
        .collect();
    if !groups.is_empty() {
        for node in nodes.iter_mut().filter(|n| n.kind == NodeKind::Note) {
            let (Some(meta), Some(text)) = (cache.file(&node.id), texts.get(&node.id)) else {
                continue;
            };
            let (tags, blocks, sections, tasks) = inputs(meta, text);
            let file = SearchFile {
                path: &node.id,
                content: text,
                tags: &tags,
                frontmatter: meta.frontmatter.as_ref(),
                blocks: &blocks,
                sections: &sections,
                tasks: &tasks,
            };
            node.color = groups
                .iter()
                .find(|(s, _)| s.matches(&file).is_some())
                .map(|(_, c)| *c);
        }
    }

    let mut edges: Vec<(u32, u32)> = edge_set.into_iter().collect();
    edges.sort();

    // Local graph: the center and its neighbors at depth 1 in either direction.
    if let Some(center) = local {
        let Some(&c) = index.get(center) else {
            return Built {
                nodes: Vec::new(),
                edges: Vec::new(),
                neighbors: Vec::new(),
            };
        };
        let mut keep: HashSet<usize> = HashSet::new();
        let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
        queue.push_back((c, 0));
        keep.insert(c);
        while let Some((n, depth)) = queue.pop_front() {
            if depth >= 1 {
                continue;
            }
            for &(a, b) in &edges {
                let (a, b) = (a as usize, b as usize);
                let other = if a == n {
                    b
                } else if b == n {
                    a
                } else {
                    continue;
                };
                if keep.insert(other) {
                    queue.push_back((other, depth + 1));
                }
            }
        }
        return finish(nodes, edges, Some(keep), true);
    }
    finish(nodes, edges, None, show_orphans)
}

/// Drops nodes outside `keep` and, unless `show_orphans`, nodes without
/// edges; renumbers edges and counts degrees.
fn finish(
    nodes: Vec<Node>,
    edges: Vec<(u32, u32)>,
    keep: Option<HashSet<usize>>,
    show_orphans: bool,
) -> Built {
    let mut degree = vec![0usize; nodes.len()];
    for &(a, b) in &edges {
        degree[a as usize] += 1;
        degree[b as usize] += 1;
    }
    let mut remap = vec![usize::MAX; nodes.len()];
    let mut out_nodes = Vec::new();
    for (i, mut node) in nodes.into_iter().enumerate() {
        let kept = keep.as_ref().is_none_or(|k| k.contains(&i));
        let orphan = degree[i] == 0 && node.kind == NodeKind::Note;
        if !kept || (orphan && !show_orphans) {
            continue;
        }
        node.degree = degree[i];
        remap[i] = out_nodes.len();
        out_nodes.push(node);
    }
    let mut out_edges = Vec::new();
    let mut neighbors = vec![Vec::new(); out_nodes.len()];
    for (a, b) in edges {
        let (a, b) = (remap[a as usize], remap[b as usize]);
        if a == usize::MAX || b == usize::MAX {
            continue;
        }
        out_edges.push((a as u32, b as u32));
        neighbors[a].push(b);
        neighbors[b].push(a);
    }
    for (i, node) in out_nodes.iter_mut().enumerate() {
        node.degree = neighbors[i].len();
    }
    Built {
        nodes: out_nodes,
        edges: out_edges,
        neighbors,
    }
}

impl Render for GraphView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let entity = cx.entity();
        let paint_entity = entity.clone();
        let node_count = self.nodes.len();
        let edge_count = self.edges.len();
        let local = self.is_local();
        let orphans = self.config.show_orphans.unwrap_or(true);
        let attachments = self.config.show_attachments.unwrap_or(false);
        let tags = self.config.show_tags.unwrap_or(false);
        let hovered_label = self
            .hovered
            .and_then(|ix| self.nodes.get(ix))
            .map(|n| n.label.clone());
        let status: SharedString = if self.building {
            "Building graph…".into()
        } else {
            format!("{node_count} nodes, {edge_count} links").into()
        };

        let chip = |label: SharedString, on: bool, theme: &Theme| {
            div()
                .px_2()
                .py_px()
                .rounded_sm()
                .text_size(px(11.))
                .cursor_pointer()
                .bg(if on {
                    theme.bg_active
                } else {
                    theme.bg_secondary
                })
                .text_color(if on { theme.text } else { theme.text_muted })
                .border_1()
                .border_color(theme.border)
                .child(label)
        };

        div()
            .key_context("GraphView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::zoom_in))
            .on_action(cx.listener(Self::zoom_out))
            .on_action(cx.listener(Self::reset_view))
            .relative()
            .size_full()
            .bg(theme.bg)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        entity.update(cx, |view, _| view.bounds = bounds);
                        let _ = window;
                    },
                    move |bounds, _, window, cx| {
                        let theme = cx.global::<Theme>().clone();
                        let line_height = window.line_height();
                        let (
                            nodes,
                            edges,
                            positions,
                            radii,
                            hovered,
                            current_ix,
                            highlight,
                            scale,
                            line_mult,
                            label_from,
                        ) = {
                            let view = paint_entity.read(cx);
                            let layout = view.layout.lock().unwrap_or_else(|e| e.into_inner());
                            let positions: Vec<Point<Pixels>> = layout
                                .bodies
                                .iter()
                                .map(|b| view.to_screen(b.x, b.y))
                                .collect();
                            let radii: Vec<f32> =
                                view.nodes.iter().map(|n| view.node_radius(n)).collect();
                            let hovered = view.hovered;
                            let current_ix = view
                                .current
                                .as_ref()
                                .and_then(|c| view.index.get(c))
                                .copied();
                            let highlight: HashSet<usize> = match hovered {
                                Some(h) => {
                                    let mut s: HashSet<usize> = view
                                        .neighbors
                                        .get(h)
                                        .cloned()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .collect();
                                    s.insert(h);
                                    s
                                }
                                None => HashSet::new(),
                            };
                            (
                                view.nodes.clone(),
                                view.edges.clone(),
                                positions,
                                radii,
                                hovered,
                                current_ix,
                                highlight,
                                view.scale,
                                view.config.line_size_multiplier.unwrap_or(1.0),
                                view.label_threshold(),
                            )
                        };
                        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                            // Edges: one stroked path for plain ones, one for highlighted.
                            let mut plain = PathBuilder::stroke(px(1.0 * line_mult));
                            let mut bright = PathBuilder::stroke(px(1.5 * line_mult));
                            let mut any_plain = false;
                            let mut any_bright = false;
                            for &(a, b) in edges.iter() {
                                let (pa, pb) = (positions[a as usize], positions[b as usize]);
                                if !bounds.contains(&pa) && !bounds.contains(&pb) {
                                    continue;
                                }
                                let on = hovered.is_some()
                                    && highlight.contains(&(a as usize))
                                    && highlight.contains(&(b as usize));
                                if on {
                                    bright.move_to(pa);
                                    bright.line_to(pb);
                                    any_bright = true;
                                } else {
                                    plain.move_to(pa);
                                    plain.line_to(pb);
                                    any_plain = true;
                                }
                            }
                            let mut edge_color = theme.text_faint;
                            edge_color.a = if hovered.is_some() { 0.25 } else { 0.6 };
                            if any_plain {
                                if let Ok(path) = plain.build() {
                                    window.paint_path(path, edge_color);
                                }
                            }
                            if any_bright {
                                if let Ok(path) = bright.build() {
                                    window.paint_path(path, theme.accent);
                                }
                            }
                            // Nodes.
                            let style = window.text_style();
                            let font_size = px(11.);
                            let mut labels = 0;
                            for (i, node) in nodes.iter().enumerate() {
                                let p = positions[i];
                                let r = px(radii[i] * scale.clamp(0.35, 1.6));
                                if p.x + r < bounds.left()
                                    || p.x - r > bounds.right()
                                    || p.y + r < bounds.top()
                                    || p.y - r > bounds.bottom()
                                {
                                    continue;
                                }
                                let dim = hovered.is_some() && !highlight.contains(&i);
                                let mut color = match node.kind {
                                    NodeKind::Note => node.color.unwrap_or(theme.text_muted),
                                    NodeKind::Attachment => theme.text_faint,
                                    NodeKind::Unresolved => theme.text_faint,
                                    NodeKind::Tag => theme.tag,
                                };
                                if Some(i) == current_ix || Some(i) == hovered {
                                    color = theme.accent;
                                }
                                if dim {
                                    color.a *= 0.3;
                                }
                                let d = r * 2.;
                                let mut quad =
                                    fill(Bounds::new(point(p.x - r, p.y - r), size(d, d)), color);
                                quad.corner_radii = gpui::Corners::all(r);
                                if node.kind == NodeKind::Unresolved {
                                    quad.background = gpui::transparent_black().into();
                                    quad.border_widths = gpui::Edges::all(px(1.));
                                    quad.border_color = color;
                                }
                                window.paint_quad(quad);
                                // Labels fade in with zoom; the hovered node always shows.
                                let show_label = Some(i) == hovered
                                    || Some(i) == current_ix
                                    || (scale >= label_from && labels < 400 && !dim);
                                if show_label {
                                    labels += 1;
                                    let mut run = style.to_run(node.label.len());
                                    run.color = if dim { theme.text_faint } else { theme.text };
                                    let line = window.text_system().shape_line(
                                        node.label.clone(),
                                        font_size,
                                        &[run],
                                        None,
                                    );
                                    let origin = point(p.x - line.width / 2., p.y + r + px(3.));
                                    let _ = line.paint(
                                        origin,
                                        line_height,
                                        TextAlign::Left,
                                        None,
                                        window,
                                        cx,
                                    );
                                }
                            }
                        });

                        let e = paint_entity.clone();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble
                                && event.button == MouseButton::Left
                                && bounds.contains(&event.position)
                            {
                                e.update(cx, |v, cx| v.mouse_down(event, window, cx));
                            }
                        });
                        let e = paint_entity.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble {
                                e.update(cx, |v, cx| v.mouse_move(event, cx));
                            }
                        });
                        let e = paint_entity.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble {
                                e.update(cx, |v, cx| v.mouse_up(event, cx));
                            }
                        });
                        let e = paint_entity.clone();
                        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                                e.update(cx, |v, cx| v.scroll(event, line_height, cx));
                            }
                        });
                    },
                )
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .top_2()
                    .left_2()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("graph-local")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_local(cx)))
                            .child(chip(
                                if local {
                                    "Local".into()
                                } else {
                                    "Global".into()
                                },
                                local,
                                &theme,
                            )),
                    )
                    .child(
                        div()
                            .id("graph-orphans")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_orphans(cx)))
                            .child(chip("Orphans".into(), orphans, &theme)),
                    )
                    .child(
                        div()
                            .id("graph-attachments")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_attachments(cx)))
                            .child(chip("Attachments".into(), attachments, &theme)),
                    )
                    .child(
                        div()
                            .id("graph-tags")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_tags(cx)))
                            .child(chip("Tags".into(), tags, &theme)),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .child(status),
                    )
                    .children(
                        hovered_label
                            .map(|l| div().text_size(px(11.)).text_color(theme.text).child(l)),
                    ),
            )
    }
}
