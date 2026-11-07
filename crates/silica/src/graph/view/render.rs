//! The graph view's paint code: drawing nodes, edges and the overlay
//! chrome — the methods of the split-out render impl, unchanged apart from
//! living in their own module. Split from `graph/view.rs` per the audit's
//! module-size finding.

use super::*;

impl Render for GraphView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // * A group field that stopped being drawn while it held the keyboard
        //   would leave the keys outside every context; the graph takes them.
        if self.editing_group.is_none() && self.group_field.focus_handle(cx).is_focused(window) {
            window.focus(&self.focus_handle, cx);
        }
        let theme = cx.global::<Theme>().clone();
        let entity = cx.entity();
        let paint_entity = entity.clone();
        let node_count = self.nodes.len();
        let edge_count = self.edges.len();
        let local = self.is_local();
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
                        entity.update(cx, |view, _| {
                            let sized =
                                view.bounds.size.width > px(0.) && view.bounds.size.height > px(0.);
                            view.bounds = bounds;
                            // * fit() does nothing until the element has a size,
                            //   so a layout that settles before the first paint
                            //   would never be framed at all.
                            if !sized && !view.user_adjusted {
                                view.fit();
                            }
                        });
                        // * The controls panel occludes: a press on one of its
                        //   sliders must not also pan the graph underneath.
                        window.insert_hitbox(bounds, HitboxBehavior::Normal)
                    },
                    move |bounds, hitbox, window, cx| {
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
                            arrows,
                            label_cache,
                        ) = {
                            let view = paint_entity.read(cx);
                            let label_cache = view.labels.clone();
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
                                view.config.line_size_multiplier.unwrap_or(1.),
                                view.label_threshold(),
                                view.config.show_arrow.unwrap_or(false),
                                label_cache,
                            )
                        };
                        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                            // * Edges: one stroked path for plain ones, one for highlighted.
                            let mut plain = PathBuilder::stroke(px(1.0 * line_mult));
                            let mut bright = PathBuilder::stroke(px(1.5 * line_mult));
                            // * Arrowheads, filled, at the target end of each
                            //   edge just outside the node.
                            let mut plain_heads = PathBuilder::fill();
                            let mut bright_heads = PathBuilder::fill();
                            let mut any_plain = false;
                            let mut any_bright = false;
                            let node_scale = scale.clamp(0.35, 1.6);
                            for &(a, b) in edges.iter() {
                                let (pa, pb) = (positions[a as usize], positions[b as usize]);
                                if !bounds.contains(&pa) && !bounds.contains(&pb) {
                                    continue;
                                }
                                let on = hovered.is_some()
                                    && highlight.contains(&(a as usize))
                                    && highlight.contains(&(b as usize));
                                let (line, heads) = if on {
                                    any_bright = true;
                                    (&mut bright, &mut bright_heads)
                                } else {
                                    any_plain = true;
                                    (&mut plain, &mut plain_heads)
                                };
                                line.move_to(pa);
                                line.line_to(pb);
                                if arrows {
                                    let r = radii[b as usize] * node_scale;
                                    if let Some([tip, left, right]) =
                                        arrowhead(pa, pb, r, 4.0 + 2.0 * line_mult)
                                    {
                                        heads.move_to(tip);
                                        heads.line_to(left);
                                        heads.line_to(right);
                                        heads.close();
                                    }
                                }
                            }
                            let mut edge_color = theme.text_faint;
                            edge_color.a = if hovered.is_some() { 0.25 } else { 0.6 };
                            if any_plain {
                                if let Ok(path) = plain.build() {
                                    window.paint_path(path, edge_color);
                                }
                                if arrows {
                                    if let Ok(path) = plain_heads.build() {
                                        window.paint_path(path, edge_color);
                                    }
                                }
                            }
                            if any_bright {
                                if let Ok(path) = bright.build() {
                                    window.paint_path(path, theme.accent);
                                }
                                if arrows {
                                    if let Ok(path) = bright_heads.build() {
                                        window.paint_path(path, theme.accent);
                                    }
                                }
                            }
                            // * Nodes.
                            let style = window.text_style();
                            let family = style.font().family.clone();
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
                                // * Labels fade in with zoom; the hovered node always shows.
                                let show_label = Some(i) == hovered
                                    || Some(i) == current_ix
                                    || (scale >= label_from && labels < 400 && !dim);
                                if show_label {
                                    labels += 1;
                                    // * The shaped line is cached: every
                                    //   paint after the first is a lookup,
                                    //   which is what keeps a large graph
                                    //   off the text rasterizer.
                                    let run_color = if dim { theme.text_faint } else { theme.text };
                                    let key = (
                                        node.label.to_string(),
                                        [
                                            run_color.h.to_bits(),
                                            run_color.s.to_bits(),
                                            run_color.l.to_bits(),
                                            run_color.a.to_bits(),
                                        ],
                                        family.clone(),
                                    );
                                    let mut cache =
                                        label_cache.lock().unwrap_or_else(|e| e.into_inner());
                                    let line = cache.entry(key).or_insert_with(|| {
                                        let mut run = style.to_run(node.label.len());
                                        run.color = run_color;
                                        window.text_system().shape_line(
                                            node.label.clone(),
                                            font_size,
                                            &[run],
                                            None,
                                        )
                                    });
                                    let line = line.clone();
                                    drop(cache);
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
                        let hb = hitbox.clone();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble
                                && event.button == MouseButton::Left
                                && hb.is_hovered(window)
                            {
                                e.update(cx, |v, cx| v.mouse_down(event, window, cx));
                            }
                        });
                        let e = paint_entity.clone();
                        let hb = hitbox.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble {
                                // * Over the panel nothing underneath is hovered.
                                let over = hb.is_hovered(window);
                                e.update(cx, |v, cx| {
                                    if over || v.drag.is_some() {
                                        v.mouse_move(event, cx);
                                    } else if v.hovered.take().is_some() {
                                        cx.notify();
                                    }
                                });
                            }
                        });
                        let e = paint_entity.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble {
                                e.update(cx, |v, cx| v.mouse_up(event, cx));
                            }
                        });
                        let e = paint_entity.clone();
                        window.on_mouse_event(
                            move |event: &ScrollWheelEvent, phase, window, cx| {
                                if phase == DispatchPhase::Bubble
                                    && hitbox.should_handle_scroll(window)
                                {
                                    e.update(cx, |v, cx| v.scroll(event, line_height, cx));
                                }
                            },
                        );
                    },
                )
                .size_full(),
            )
            .child(self.render_controls(&theme, cx))
            .child(
                div()
                    .absolute()
                    .top_2()
                    .right_2()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .children(
                        hovered_label
                            .map(|l| div().text_size(px(11.)).text_color(theme.text).child(l)),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .child(status),
                    )
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
                    ),
            )
    }
}

/// The three corners of an arrowhead on the edge from `from` to `to`, its
/// tip `radius` short of `to` so it sits on the node's rim; `None` for an
/// edge too short to carry one.
pub(crate) fn arrowhead(
    from: Point<Pixels>,
    to: Point<Pixels>,
    radius: f32,
    length: f32,
) -> Option<[Point<Pixels>; 3]> {
    let dx = f32::from(to.x) - f32::from(from.x);
    let dy = f32::from(to.y) - f32::from(from.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= radius + length {
        return None;
    }
    let (ux, uy) = (dx / len, dy / len);
    let tip = (f32::from(to.x) - ux * radius, f32::from(to.y) - uy * radius);
    let base = (tip.0 - ux * length, tip.1 - uy * length);
    let half = length * 0.5;
    let left = (base.0 - uy * half, base.1 + ux * half);
    let right = (base.0 + uy * half, base.1 - ux * half);
    Some([
        point(px(tip.0), px(tip.1)),
        point(px(left.0), px(left.1)),
        point(px(right.0), px(right.1)),
    ])
}
