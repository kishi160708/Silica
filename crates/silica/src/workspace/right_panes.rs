//! The right-dock panes: Tags, All properties, Outline, plus the filter
//! field, scrolling and context menus they share — the methods of the
//! split-out section, unchanged apart from living in their own module.
//! Split from `workspace.rs` per the audit's module-size finding.

use super::*;

actions!(pane_keys, [PaneUp, PaneDown, PaneActivate, PaneLeaveFocus]);

impl Workspace {

    /// Obsidian's `nav-header`: 40 px, 8 px of padding, the buttons
    /// centred 2 px apart.
    pub(crate) fn pane_header() -> gpui::Div {
        div()
            .flex_shrink_0()
            .h(px(HEADER_HEIGHT))
            .p(px(8.))
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(2.))
    }

    /// The filter field of a pane, by pane.
    pub(crate) fn pane_filter(&self, which: RightTab) -> &Entity<TextInput> {
        match which {
            RightTab::Tags => &self.tag_filter,
            RightTab::Properties => &self.property_filter,
            RightTab::Outline => &self.outline_filter,
            _ => &self.backlink_filter,
        }
    }

    pub(crate) fn pane_show_search(&mut self, which: RightTab) -> &mut bool {
        match which {
            RightTab::Tags => &mut self.tag_state.show_search,
            RightTab::Properties => &mut self.property_state.show_search,
            RightTab::Outline => &mut self.outline_state.show_search,
            _ => &mut self.backlink_state.show_search,
        }
    }

    /// "Show search filter": shows the box and puts the keyboard in it, or
    /// hides it again, leaving the query (and the rows) as they were.
    pub(crate) fn toggle_pane_search(&mut self, which: RightTab, window: &mut Window, cx: &mut Context<Self>) {
        let flag = self.pane_show_search(which);
        *flag = !*flag;
        if *flag {
            let handle = self.pane_filter(which).focus_handle(cx);
            window.focus(&handle, cx);
        }
        cx.notify();
    }

    /// Escape in the box: hides it and clears the query, so every row is
    /// back, and the keyboard goes to the note.
    pub(crate) fn hide_pane_search(&mut self, which: RightTab, window: &mut Window, cx: &mut Context<Self>) {
        *self.pane_show_search(which) = false;
        self.pane_filter(which)
            .update(cx, |input, cx| input.set_text("", cx));
        self.focus_main(window, cx);
        cx.notify();
    }

    /// The box's clear button: empties it and keeps the keyboard there.
    pub(crate) fn clear_pane_filter(&mut self, which: RightTab, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.pane_filter(which).clone();
        input.update(cx, |input, cx| input.set_text("", cx));
        window.focus(&input.focus_handle(cx), cx);
        cx.notify();
    }

    pub(crate) fn toggle_tag_hierarchy(&mut self, cx: &mut Context<Self>) {
        self.tag_state.use_hierarchy = !self.tag_state.use_hierarchy;
        // * Obsidian's tree comes back with every folder open.
        self.tag_state.collapsed.clear();
        self.tag_state.tag_fold_next = false;
        cx.notify();
    }

    /// "Expand all" (whose label never changes in Obsidian): folds every
    /// tag, leaves included, or opens them all again.
    pub(crate) fn toggle_tag_folds(&mut self, cx: &mut Context<Self>) {
        if self.tag_state.tag_fold_next {
            self.tag_state.collapsed = panes::tag_keys(&self.tag_tree).into_iter().collect();
        } else {
            self.tag_state.collapsed.clear();
        }
        self.tag_state.tag_fold_next = !self.tag_state.tag_fold_next;
        cx.notify();
    }

    pub(crate) fn toggle_tag_fold(&mut self, key: String, cx: &mut Context<Self>) {
        if !self.tag_state.collapsed.remove(&key) {
            self.tag_state.collapsed.insert(key);
        }
        cx.notify();
    }

    pub(crate) fn set_pane_sort(&mut self, which: RightTab, order: PaneSort, cx: &mut Context<Self>) {
        match which {
            RightTab::Tags => {
                self.tag_state.sort = order;
                self.tag_state.sort_menu_open = false;
                panes::sort_tags(&mut self.tag_tree, order);
            }
            _ => {
                self.property_state.sort = order;
                self.property_state.sort_menu_open = false;
            }
        }
        cx.notify();
    }

    pub(crate) fn toggle_outline_fold(&mut self, ix: usize, cx: &mut Context<Self>) {
        if !self.outline_state.collapsed.remove(&ix) {
            self.outline_state.collapsed.insert(ix);
        }
        cx.notify();
    }

    /// The headings that have children: what "Collapse all" folds.
    pub(crate) fn outline_parents(&self) -> Vec<usize> {
        let depths = panes::outline_depths(&self.outline);
        (0..depths.len())
            .filter(|&ix| depths.get(ix + 1).is_some_and(|&d| d > depths[ix]))
            .collect()
    }

    /// The top-level headings with children: what "Collapse all" folds,
    /// leaving the folds under them as they were, and what "Expand all"
    /// counts.
    pub(crate) fn outline_top_parents(&self) -> Vec<usize> {
        let depths = panes::outline_depths(&self.outline);
        self.outline_parents()
            .into_iter()
            .filter(|&ix| depths[ix] == 0)
            .collect()
    }

    /// Whether every top-level heading with children is folded, which is
    /// when the button reads "Expand all".
    pub(crate) fn outline_all_folded(&self) -> bool {
        let parents = self.outline_top_parents();
        !parents.is_empty()
            && parents
                .iter()
                .all(|ix| self.outline_state.collapsed.contains(ix))
    }

    /// "Collapse all" folds the top-level headings and leaves the deeper
    /// folds alone; "Expand all" opens everything.
    pub(crate) fn toggle_outline_folds(&mut self, cx: &mut Context<Self>) {
        if self.outline_all_folded() {
            self.outline_state.collapsed.clear();
        } else {
            let top = self.outline_top_parents();
            self.outline_state.collapsed.extend(top);
        }
        cx.notify();
    }

    pub(crate) fn toggle_follow_cursor(&mut self, cx: &mut Context<Self>) {
        self.outline_state.follow_cursor = !self.outline_state.follow_cursor;
        self.outline_state.scroll_pending = true;
        cx.notify();
    }

    /// An Outline row's click: in an editing mode the cursor goes to the
    /// heading's line, column 0, scrolled to the vertical centre, and the
    /// editor takes the keyboard; in reading mode the heading scrolls to
    /// the top edge.
    pub(crate) fn reveal_heading(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.reading
            .update(cx, |r, cx| r.scroll_to_offset(offset, cx));
        if self.mode.is_editing() {
            self.editor
                .update(cx, |e, cx| e.reveal_offset_centered(offset, cx));
        } else {
            self.editor.update(cx, |e, cx| e.reveal_offset(offset, cx));
        }
        self.request_focus_main(cx);
    }

    /// The heading whose section holds the cursor (editing) or the block at
    /// the top of the view (reading); none above the first heading.
    pub(crate) fn current_heading(&self, cx: &App) -> Option<usize> {
        let path = self.current.as_deref()?;
        let offset = match self.mode {
            ViewMode::Reading => self.reading.read(cx).top_offset()?,
            mode if mode.is_editing() => {
                let editor = self.editor.read(cx);
                if editor.name.as_ref() != path {
                    return None;
                }
                editor.selection().start
            }
            _ => return None,
        };
        panes::heading_at(&self.outline, offset)
    }

    /// Brings the Outline's mark up to date when the cursor or the reading
    /// view moved, and asks for a draw only when the mark moved.
    pub(crate) fn follow_heading(&mut self, cx: &mut Context<Self>) {
        if !self.right_visible || self.right_tab != RightTab::Outline {
            return;
        }
        let active = self.current_heading(cx);
        if active != self.outline_state.active {
            self.outline_state.active = active;
            self.outline_state.scroll_pending = true;
            cx.notify();
        }
    }

    /// The Tags pane's rows as drawn now.
    pub(crate) fn tag_rows(&self, cx: &App) -> Vec<panes::TagRow> {
        panes::tag_rows(
            &self.tag_tree,
            self.tag_state.sort,
            self.tag_state.use_hierarchy,
            &self.tag_state.collapsed,
            self.tag_filter.read(cx).text(),
        )
    }

    pub(crate) fn property_rows(&self, cx: &App) -> Vec<panes::PropertyRow> {
        panes::property_rows(
            &self.property_entries,
            self.property_state.sort,
            self.property_filter.read(cx).text(),
        )
    }

    pub(crate) fn outline_rows(&self, cx: &App) -> Vec<panes::OutlineRow> {
        panes::outline_rows(
            &self.outline,
            &self.outline_state.collapsed,
            self.outline_filter.read(cx).text(),
            self.outline_state.active,
        )
    }

    /// The search box under a pane's header: Obsidian's
    /// `search-input-container`, 30 px tall with 4 px above and below it
    /// inside the header, so the rows move down 38 px, and a round clear
    /// button at its right end once there is text.
    pub(crate) fn render_pane_filter(
        &self,
        which: RightTab,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let input = self.pane_filter(which).clone();
        let has_text = !input.read(cx).text().is_empty();
        // * Obsidian's `search-input-container`: the field's text starts
        //   30 px in, behind a magnifier at the left.
        div().flex_shrink_0().px(px(8.)).pb(px(8.)).child(
            div()
                .relative()
                .w_full()
                .h(px(30.))
                .child(input)
                .child(
                    div()
                        .absolute()
                        .left(px(8.))
                        .top(px(0.))
                        .h(px(30.))
                        .flex()
                        .items_center()
                        .child(icon("search", 14., theme.text_muted)),
                )
                .children(has_text.then(|| {
                    div()
                        .id(("pane-filter-clear", which as usize))
                        .absolute()
                        .right(px(2.))
                        .top(px(0.))
                        .w(px(28.))
                        .h(px(30.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .rounded_full()
                        .tooltip(tooltip("Clear search"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.clear_pane_filter(which, window, cx)
                        }))
                        .child(icon("x", 14., theme.text_muted))
                })),
        )
    }

    /// "Change sort order" for the Tags and All properties panes: the two
    /// name orders, a separator, the two frequency orders, a check against
    /// the one in force.
    pub(crate) fn render_pane_sort_menu(
        &mut self,
        which: RightTab,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let (open, current, noun, id) = match which {
            RightTab::Tags => (
                self.tag_state.sort_menu_open,
                self.tag_state.sort,
                "Tag name",
                "tags-sort-menu",
            ),
            _ => (
                self.property_state.sort_menu_open,
                self.property_state.sort,
                "Property name",
                "properties-sort-menu",
            ),
        };
        if !open {
            return None;
        }
        let mut list = div()
            .id(id)
            .flex()
            .flex_col()
            .min_w(px(196.))
            .py_1()
            .bg(theme.bg_secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_lg()
            .text_size(px(13.))
            .text_color(theme.text)
            .occlude()
            .on_mouse_down_out(cx.listener(move |this, _, _, cx| {
                match which {
                    RightTab::Tags => this.tag_state.sort_menu_open = false,
                    _ => this.property_state.sort_menu_open = false,
                }
                cx.notify();
            }));
        for (ix, order) in PaneSort::ALL.into_iter().enumerate() {
            if ix == 2 {
                list = list.child(div().my_1().h(px(1.)).bg(theme.border));
            }
            let checked = order == current;
            list = list.child(
                div()
                    .id((id, ix))
                    .h(px(25.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.set_pane_sort(which, order, cx)),
                    )
                    .child(
                        div()
                            .w(px(16.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .children(checked.then(|| icon("check", 14., theme.text_muted))),
                    )
                    .child(SharedString::from(order.label(noun))),
            );
        }
        Some(
            deferred(
                anchored()
                    .position(self.sort_menu_at)
                    .anchor(Anchor::TopRight)
                    .snap_to_window_with_margin(px(8.))
                    .child(list),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }

    /// Opens the side-pane context menu at the pointer's position, as
    /// Obsidian's right-click on a Tags/All properties/Outline row does.
    pub(crate) fn open_side_menu(
        &mut self,
        position: gpui::Point<Pixels>,
        target: SidePaneTarget,
        cx: &mut Context<Self>,
    ) {
        self.side_menu_at = position;
        self.side_menu = Some(SidePaneMenu { position, target });
        cx.notify();
    }

    /// Closes the open context menu, as clicking outside one does.
    pub(crate) fn close_side_menu(&mut self, cx: &mut Context<Self>) {
        if self.side_menu.take().is_some() {
            cx.notify();
        }
    }

    /// The right-click context menu over a side-pane row.
    pub(crate) fn render_side_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.side_menu.as_ref()?;
        let items = side_menu_items(&menu.target);
        let mut list = div()
            .id("side-menu")
            .flex()
            .flex_col()
            .min_w(px(196.))
            .py_1()
            .bg(theme.bg_secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_lg()
            .text_size(px(13.))
            .text_color(theme.text)
            .occlude()
            .on_mouse_down_out(cx.listener(move |this, _, _, cx| {
                this.close_side_menu(cx);
            }));
        for (ix, item) in items.into_iter().enumerate() {
            if ix > 0 {
                list = list.child(div().my_1().h(px(1.)).bg(theme.border));
            }
            list = list.child(
                div()
                    .id(("side-menu-item", ix))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.close_side_menu(cx);
                        this.handle_side_menu_action(&item.action, window, cx);
                        cx.notify();
                    }))
                    .child(crate::assets::icon(item.icon, 16., theme.text_muted))
                    .child(item.label),
            );
        }
        Some(
            deferred(
                anchored()
                    .position(menu.position)
                    .anchor(Anchor::TopLeft)
                    .snap_to_window_with_margin(px(8.))
                    .child(list),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }

    /// Handles a side-pane context menu action: searches for tags/properties,
    /// copies paths, or opens outline headings.
    pub(crate) fn handle_side_menu_action(&mut self, action: &SidePaneAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            SidePaneAction::SearchTag(tag) => {
                self.show_search(&format!("tag:#{tag}"), None, cx);
            }
            SidePaneAction::RenameTag(tag) => {
                // * The prompt asks for the new tag name; the rename itself
                //   rewrites every occurrence across the vault on submit.
                let Some(state) = self.state.clone() else {
                    return;
                };
                let tag_lower = tag.to_lowercase();
                let count = state
                    .cache()
                    .tags()
                    .get(&tag_lower)
                    .map(|e| e.count)
                    .unwrap_or(0);
                self.open_prompt(
                    PendingOp::RenameTag { from: tag.clone() },
                    PromptSpec {
                        title: "Rename tag".into(),
                        detail: Some(
                            format!("Rename \"#{tag}\" to what? ({count} occurrence{} across the vault)", if count == 1 { "" } else { "s" }).into(),
                        ),
                        input: None,
                        placeholder: "Tag name".into(),
                        confirm: "Rename".into(),
                        ..PromptSpec::default()
                    },
                    window,
                    cx,
                );
            }
            SidePaneAction::CopyTag(tag) => {
                self.copy_path(tag, cx);
            }
            SidePaneAction::SearchProperty(key) => {
                self.show_search(&format!("[\"{key}\"]"), None, cx);
            }
            SidePaneAction::CopyProperty(key) => {
                self.copy_path(key, cx);
            }
            SidePaneAction::OpenHeading(path, offset) => {
                self.open_file(path.clone(), None, cx);
                self.reveal_offset(*offset, cx);
            }
            SidePaneAction::OpenHeadingInNewTab(path) => {
                self.open_in_new_tab(path.clone(), None, cx);
            }
            SidePaneAction::OpenHeadingToRight(path) => {
                self.open_in_new_tab(path.clone(), None, cx);
            }
            SidePaneAction::OpenLink(path) => {
                self.open_file(path.clone(), None, cx);
            }
            SidePaneAction::OpenLinkInNewTab(path) => {
                self.open_in_new_tab(path.clone(), None, cx);
            }
            SidePaneAction::OpenLinkToRight(path) => {
                self.open_in_new_tab(path.clone(), None, cx);
            }
            SidePaneAction::CopyLinkPath(path) => {
                self.copy_path(path, cx);
            }
        }
    }

    /// Obsidian's `tree-item-self`: 25 px tall at a 27 px pitch,
    /// `padding: 4px 8px 4px 24px` with 17 px more on the left per level of
    /// depth, an 8 px radius, 13 px muted text that brightens under the
    /// pointer (or when `active`, the Outline's `is-active` mark), a faint
    /// flair at the right edge, a fold triangle or an icon in the gutter
    /// 20 px before the text, and an indentation guide for every level.
    /// A `wrap` label wraps at 17 px per line; the others are cut with an
    /// ellipsis.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn tree_row(
        id: (&'static str, usize),
        depth: usize,
        fold: Option<(bool, FoldAction)>,
        icon_name: Option<&'static str>,
        label: gpui::AnyElement,
        flair: Option<SharedString>,
        active: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
        on_right_click: Option<RightClickHandler<Self>>,
    ) -> gpui::Stateful<gpui::Div> {
        let text_x = 24. + 17. * depth as f32;
        let gutter = match (fold, icon_name) {
            (Some((collapsed, on_fold)), _) => Some(
                div()
                    .id((id.0, id.1 + 1_000_000))
                    .absolute()
                    .left(px(text_x - 20.))
                    .top(px(4.))
                    .w(px(16.))
                    .h(px(17.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .opacity(0.85)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        on_fold(this, cx);
                        cx.stop_propagation();
                    }))
                    .child(
                        div()
                            .text_color(theme.text_faint)
                            .group_hover("tree-row", |s| s.text_color(theme.text_muted))
                            .child(icon(
                                if collapsed {
                                    "chevron-right"
                                } else {
                                    "chevron-down"
                                },
                                10.,
                                theme.text_faint,
                            )),
                    ),
            ),
            (None, Some(name)) => Some(
                div()
                    .id((id.0, id.1 + 1_000_000))
                    .absolute()
                    .left(px(text_x - 20.))
                    .top(px(4.))
                    .w(px(16.))
                    .h(px(17.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .opacity(0.85)
                    .child(icon(name, 14., theme.text_muted)),
            ),
            (None, None) => None,
        };
        div()
            .id(id)
            .group("tree-row")
            .relative()
            .w_full()
            .flex_shrink_0()
            .min_h(px(25.))
            .mb(px(2.))
            .pl(px(text_x))
            .pr(px(8.))
            .py(px(4.))
            .flex()
            .flex_row()
            .items_start()
            .rounded(px(8.))
            .cursor_pointer()
            .when(active, |d| d.bg(theme.bg_alt))
            .hover(|s| s.bg(theme.bg_alt))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    if let Some(on_right_click) = &on_right_click {
                        cx.stop_propagation();
                        on_right_click(this, event.position, cx);
                    }
                }),
            )
            // * Obsidian's `tree-item-children` guides: a hairline 12 px in
            //   from each ancestor's text, the row's full pitch tall.
            .children((1..=depth).map(|k| {
                div()
                    .absolute()
                    .left(px(17. * k as f32 - 5.))
                    .top(px(0.))
                    .bottom(px(-2.))
                    .w(px(1.))
                    .bg(theme.border)
            }))
            .children(gutter)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(FONT_UI_SMALL))
                    .line_height(px(17.))
                    .text_color(if active { theme.text } else { theme.text_muted })
                    .group_hover("tree-row", |s| s.text_color(theme.text))
                    .child(label),
            )
            .children(flair.map(|count| {
                div()
                    .flex_shrink_0()
                    .pl(px(4.))
                    .pt(px(3.))
                    .text_size(px(FONT_UI_SMALLER))
                    .line_height(px(12.))
                    .text_color(theme.text_faint)
                    .group_hover("tree-row", |s| s.text_color(theme.text_muted))
                    .child(count)
            }))
    }

    /// A one-line label cut with an ellipsis.
    pub(crate) fn tree_label(text: impl Into<SharedString>) -> gpui::AnyElement {
        div()
            .whitespace_nowrap()
            .overflow_hidden()
            .text_ellipsis()
            .child(text.into())
            .into_any_element()
    }

    /// The Tags pane: sort, nested, expand-all and search buttons, the
    /// filter when shown, then the tags as a tree (or a flat list) with
    /// their counts; a row's click searches `tag:#path`.
    pub(crate) fn render_tags_pane(&mut self, theme: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (hierarchy, show_search) = (self.tag_state.use_hierarchy, self.tag_state.show_search);
        let sort_menu = self.render_pane_sort_menu(RightTab::Tags, theme, cx);
        let header = Self::pane_header()
            .child(
                div()
                    .relative()
                    .child(
                        Self::nav_button(
                            "tags-sort",
                            "arrow-up-narrow-wide",
                            "Change sort order",
                            false,
                            theme,
                        )
                        .on_click(cx.listener(
                            |this, event: &ClickEvent, _, cx| {
                                this.sort_menu_at = event.position();
                                this.tag_state.sort_menu_open = !this.tag_state.sort_menu_open;
                                cx.notify();
                            },
                        )),
                    )
                    .children(sort_menu),
            )
            .child(
                Self::nav_button(
                    "tags-nested",
                    "folder-tree",
                    "Show nested tags",
                    hierarchy,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.toggle_tag_hierarchy(cx))),
            )
            .child(
                Self::nav_button(
                    "tags-expand",
                    "chevrons-up-down",
                    "Expand all",
                    false,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.toggle_tag_folds(cx))),
            )
            .child(
                Self::nav_button(
                    "tags-search",
                    "search",
                    "Show search filter",
                    show_search,
                    theme,
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.toggle_pane_search(RightTab::Tags, window, cx)
                })),
            );
        let filter = show_search.then(|| self.render_pane_filter(RightTab::Tags, theme, cx));
        let rows = self.tag_rows(cx);
        // * Obsidian's `.tag-container`: padding 12px 12px 32px.
        let mut list = div()
            .id("tags")
            .flex()
            .flex_col()
            .flex_1()
            .overflow_y_scroll()
            .px(px(12.))
            .pt(px(12.))
            .pb(px(32.));
        for (ix, row) in rows.into_iter().enumerate() {
            let path = row.path.clone();
            let path_for_menu = path.clone();
            let fold = row.has_children.then(|| {
                let key = row.key.clone();
                let on_fold: FoldAction =
                    Box::new(move |this, cx| this.toggle_tag_fold(key.clone(), cx));
                (row.collapsed, on_fold)
            });
            // * The parent part shows only in flat mode, in the same colour.
            let label = if hierarchy {
                row.name.clone()
            } else {
                row.path.clone()
            };
            list = list.child(
                Self::tree_row(
                    ("tag-row", ix),
                    row.depth,
                    fold,
                    None,
                    Self::tree_label(label),
                    Some(row.count.to_string().into()),
                    self.tag_state.selected == Some(ix),
                    theme,
                    cx,
                    Some(Box::new(move |this, position, cx| {
                        this.open_side_menu(
                            position,
                            SidePaneTarget::Tag(path_for_menu.clone()),
                            cx,
                        );
                    })),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.show_search(&format!("tag:#{path}"), Some(window), cx);
                    this.focus_panes(window, cx);
                })),
            );
        }
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(header)
            .children(filter)
            .child(list)
            .into_any_element()
    }

    /// The All properties pane: sort and search buttons, the filter when
    /// shown, then one row per key with its widget's icon and the notes
    /// carrying it; a row's click searches `["key"]`.
    pub(crate) fn render_properties_pane(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let show_search = self.property_state.show_search;
        let sort_menu = self.render_pane_sort_menu(RightTab::Properties, theme, cx);
        let header = Self::pane_header()
            .child(
                div()
                    .relative()
                    .child(
                        Self::nav_button(
                            "properties-sort",
                            "arrow-up-narrow-wide",
                            "Change sort order",
                            false,
                            theme,
                        )
                        .on_click(cx.listener(
                            |this, event: &ClickEvent, _, cx| {
                                this.sort_menu_at = event.position();
                                this.property_state.sort_menu_open =
                                    !this.property_state.sort_menu_open;
                                cx.notify();
                            },
                        )),
                    )
                    .children(sort_menu),
            )
            .child(
                Self::nav_button(
                    "properties-search",
                    "search",
                    "Show search filter",
                    show_search,
                    theme,
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.toggle_pane_search(RightTab::Properties, window, cx)
                })),
            );
        let filter = show_search.then(|| self.render_pane_filter(RightTab::Properties, theme, cx));
        let rows = self.property_rows(cx);
        // * Obsidian's `.view-content`: padding 4px 12px 32px.
        let mut list = div()
            .id("properties")
            .flex()
            .flex_col()
            .flex_1()
            .overflow_y_scroll()
            .px(px(12.))
            .pt(px(4.))
            .pb(px(32.));
        for (ix, row) in rows.into_iter().enumerate() {
            let name = row.name.clone();
            let name_for_menu = name.clone();
            list = list.child(
                Self::tree_row(
                    ("property-row", ix),
                    0,
                    None,
                    Some(row.icon),
                    Self::tree_label(name.clone()),
                    Some(row.count.to_string().into()),
                    self.property_state.selected == Some(ix),
                    theme,
                    cx,
                    Some(Box::new(move |this, position, cx| {
                        this.open_side_menu(
                            position,
                            SidePaneTarget::Property(name_for_menu.clone()),
                            cx,
                        );
                    })),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.show_search(&format!("[\\\"{name}\\\"]"), Some(window), cx);
                })),
            );
        }
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(header)
            .children(filter)
            .child(list)
            .into_any_element()
    }

    /// The Outline pane: search, auto-scroll and collapse-all buttons, the
    /// filter when shown, then the headings as a structural tree with the
    /// one holding the cursor marked, or "No headings found.".
    pub(crate) fn render_outline_pane(&mut self, theme: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        // * The mark follows the cursor and the reading view; a mode or note
        //   switch lands here before anything else can notice it.
        let active = self.current_heading(cx);
        if active != self.outline_state.active {
            self.outline_state.active = active;
            self.outline_state.scroll_pending = true;
        }
        let (follow, show_search) = (
            self.outline_state.follow_cursor,
            self.outline_state.show_search,
        );
        let all_folded = self.outline_all_folded();
        let header = Self::pane_header()
            .child(
                Self::nav_button(
                    "outline-search",
                    "search",
                    "Show search filter",
                    show_search,
                    theme,
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.toggle_pane_search(RightTab::Outline, window, cx)
                })),
            )
            .child(
                Self::nav_button(
                    "outline-follow",
                    "gallery-vertical",
                    "Auto-scroll to current section",
                    follow,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.toggle_follow_cursor(cx))),
            )
            .child(
                if all_folded {
                    Self::nav_button(
                        "outline-fold",
                        "chevrons-up-down",
                        "Expand all",
                        false,
                        theme,
                    )
                } else {
                    Self::nav_button(
                        "outline-fold",
                        "chevrons-down-up",
                        "Collapse all",
                        false,
                        theme,
                    )
                }
                .on_click(cx.listener(|this, _, _, cx| this.toggle_outline_folds(cx))),
            );
        let filter = show_search.then(|| self.render_pane_filter(RightTab::Outline, theme, cx));
        let body: gpui::AnyElement = if self.outline.is_empty() {
            // * Obsidian's `.pane-empty`: 13 px faint, padding 24px 16px,
            //   margin 8px 0, centred.
            div()
                .flex_1()
                .px(px(12.))
                .child(
                    div()
                        .my(px(8.))
                        .px(px(16.))
                        .py(px(24.))
                        .text_size(px(FONT_UI_SMALL))
                        .text_color(theme.text_faint)
                        .text_center()
                        .child("No headings found."),
                )
                .into_any_element()
        } else {
            let rows = self.outline_rows(cx);
            if follow && self.outline_state.scroll_pending {
                self.outline_state.scroll_pending = match rows.iter().position(|r| r.active) {
                    Some(ix) => !scroll_outline_to(&self.outline_state.scroll, ix),
                    None => false,
                };
            }
            let mut list = div()
                .id("outline")
                .flex()
                .flex_col()
                .flex_1()
                .overflow_y_scroll()
                .track_scroll(&self.outline_state.scroll)
                .px(px(12.))
                .pt(px(4.))
                .pb(px(32.));
            for (ix, row) in rows.into_iter().enumerate() {
                let offset = row.offset;
                let fold = row.has_children.then(|| {
                    let ix = row.ix;
                    let on_fold: FoldAction =
                        Box::new(move |this, cx| this.toggle_outline_fold(ix, cx));
                    (row.collapsed, on_fold)
                });
                // * The text wraps; a search match is Obsidian's
                //   `suggestion-highlight`, drawn in the accent.
                let highlights: Vec<(Range<usize>, HighlightStyle)> = row
                    .highlights
                    .iter()
                    .filter(|r| r.end <= row.text.len())
                    .map(|r| {
                        (
                            r.clone(),
                            HighlightStyle {
                                color: Some(theme.accent),
                                ..HighlightStyle::default()
                            },
                        )
                    })
                    .collect();
                let label = StyledText::new(SharedString::from(row.text.clone()))
                    .with_highlights(highlights)
                    .into_any_element();
                list = list.child(
                    Self::tree_row(
                        ("outline-row", row.ix),
                        row.depth,
                        fold,
                        None,
                        label,
                        None,
                        row.active || self.outline_state.selected == Some(ix),
                        theme,
                        cx,
                        Some(Box::new(move |this, position, cx| {
                            let path = this.current.clone().unwrap_or_default();
                            this.open_side_menu(
                                position,
                                SidePaneTarget::Heading {
                                    path,
                                    offset: row.offset,
                                },
                                cx,
                            );
                        })),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.reveal_heading(offset, cx))),
                );
            }
            list.into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(header)
            .children(filter)
            .child(body)
            .into_any_element()
    }

    /// Whether a left-dock pane's core plugin is on, by its id.
    pub(crate) fn left_pane_enabled(&self, id: &str) -> bool {
        self.state
            .as_ref()
            .is_none_or(|state| state.core_plugin_enabled(id))
    }

    /// Whether a right-dock pane's core plugin is on. Each of the five is
    /// its own plugin in `core-plugins.json`, and a vault with no file has
    /// them all, so the default is what `CORE_PLUGINS` says.
    pub(crate) fn pane_enabled(&self, tab: RightTab) -> bool {
        let id = match tab {
            RightTab::Backlinks => "backlink",
            RightTab::Outgoing => "outgoing-link",
            RightTab::Tags => "tag-pane",
            RightTab::Properties => "properties",
            RightTab::Outline => "outline",
        };
        self.state
            .as_ref()
            .is_none_or(|state| state.core_plugin_enabled(id))
    }

    pub(crate) fn render_right_dock(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        // * The pane whose plugin was just turned off has no tab any more,
        //   so it cannot be the one on screen either: the first that is
        //   still on takes over, as Obsidian moves to a neighbouring leaf.
        if !self.pane_enabled(self.right_tab) {
            if let Some(next) = [
                RightTab::Backlinks,
                RightTab::Outgoing,
                RightTab::Tags,
                RightTab::Properties,
                RightTab::Outline,
            ]
            .into_iter()
            .find(|t| self.pane_enabled(*t))
            {
                self.right_tab = next;
            }
        }
        let tab = self.right_tab;
        let body: gpui::AnyElement = match tab {
            RightTab::Backlinks => self.render_backlinks_pane(theme, cx),
            RightTab::Outgoing => self.render_outgoing_pane(theme, cx),
            RightTab::Outline => self.render_outline_pane(theme, cx),
            RightTab::Tags => self.render_tags_pane(theme, cx),
            RightTab::Properties => self.render_properties_pane(theme, cx),
        };
        // * A pane whose core plugin is off is not offered: Obsidian takes
        //   the view away with the plugin.
        let tab_button = |this: &mut Self,
                          label: &'static str,
                          icon_name: &'static str,
                          which: RightTab,
                          cx: &mut Context<Self>| {
            this.pane_enabled(which).then(|| {
                this.dock_tab(
                    label,
                    icon_name,
                    tab == which,
                    theme,
                    move |this, window, cx| {
                        this.select_right_tab(which, cx);
                        // * The dock tab takes the keyboard, the way a row
                        //   click does: the arrows then walk the pane.
                        this.focus_panes(window, cx);
                    },
                    cx,
                )
            })
        };
        let frameless = self.frameless;
        let mac = self.mac_frame();
        // * Obsidian's tab order and icon names (`links-coming-in` and
        //   `links-going-out` are its own drawings; Silica's are in `assets`).
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(px(self.right_width as f32))
            .h_full()
            .bg(theme.bg_secondary)
            // * The pane actions live on this container: the panes' own
            //   focus handle makes them reachable, and each handler checks
            //   that handle is focused before it acts.
            .on_action(cx.listener(Self::pane_up))
            .on_action(cx.listener(Self::pane_down))
            .on_action(cx.listener(Self::pane_on_enter))
            .on_action(cx.listener(Self::pane_on_escape))
            .map(|d| self.corners.right().apply(d))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .h(px(HEADER_HEIGHT))
                    .pl(px(8.))
                    .pr(if frameless && !mac {
                        px(3. * RIBBON_WIDTH)
                    } else {
                        px(8.)
                    })
                    .gap(px(3.))
                    .border_b_1()
                    .border_color(theme.border)
                    .children(tab_button(
                        self,
                        "Backlinks",
                        "links-coming-in",
                        RightTab::Backlinks,
                        cx,
                    ))
                    .children(tab_button(
                        self,
                        "Outgoing links",
                        "links-going-out",
                        RightTab::Outgoing,
                        cx,
                    ))
                    .children(tab_button(self, "Tags", "tags", RightTab::Tags, cx))
                    .children(tab_button(
                        self,
                        "All properties",
                        "archive",
                        RightTab::Properties,
                        cx,
                    ))
                    .children(tab_button(self, "Outline", "list", RightTab::Outline, cx))
                    .child(window_handle(
                        div().id("right-dock-space").flex_1().h_full(),
                        frameless,
                    ))
                    // * The window's top-right corner on macOS: the right
                    //   sidebar's toggle, which Obsidian fixes there.
                    .when(mac, |d| d.child(self.sidebar_toggle(false, theme, cx))),
            )
            .child(div().flex_1().overflow_hidden().child(body))
            .children(self.render_side_menu(theme, cx))
            .child(self.dock_handle(false, theme))
            .children(self.render_right_dock_context_menu(theme, cx))
    }

    /// Renders the right-dock context menu over backlinks/outgoing links/tags/outline rows.
    pub(crate) fn render_right_dock_context_menu(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let Some(path) = &self.right_dock_context_menu else {
            return None;
        };
        let entity = cx.entity();
        let path_clone = path.clone();
        let mut list = div()
            .id("right-dock-menu")
            .absolute()
            .left(px(10.))
            .top(px(40.))
            .flex()
            .flex_col()
            .min_w(px(180.))
            .py_1()
            .bg(theme.bg_secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_lg()
            .text_size(px(13.))
            .text_color(theme.text)
            .occlude();
        let items: Vec<(&'static str, &'static str)> = vec![
            ("Open in new tab", "file-plus"),
            ("Open to the right", "separator-vertical"),
            ("Copy path", "clipboard"),
        ];
        for (label, icon) in items {
            let path_for_item = path_clone.clone();
            let entity_for_item = entity.clone();
            list = list.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_0p5()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_active))
                    .child(
                        div()
                            .w(px(16.))
                            .flex_shrink_0()
                            .text_color(theme.text_muted)
                            .child(crate::assets::icon(icon, 14., theme.text_muted)),
                    )
                    .child(label)
                    .on_mouse_down(gpui::MouseButton::Left, move |_, _, cx| {
                        let path_for_item = path_for_item.clone();
                        entity_for_item.update(cx, |ws, cx| {
                            ws.right_dock_context_menu = None;
                            match label {
                                "Open in new tab" => {
                                    ws.open_file(path_for_item.clone(), None, cx);
                                }
                                "Open to the right" => {
                                    ws.open_file(path_for_item.clone(), None, cx);
                                }
                                "Copy path" => {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        path_for_item,
                                    ));
                                }
                                _ => {}
                            }
                        });
                    }),
            );
        }
        Some(list.into_any_element())
    }

    // ----- keyboard navigation -------------------------------------------------

    /// Focuses the right-dock panes, the way clicking a row or its dock tab
    /// does: the arrow keys move within the pane from then on.
    pub(crate) fn focus_panes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.pane_focus.clone();
        window.focus(&handle, cx);
        cx.notify();
    }

    /// Whether the panes hold the keyboard. The four pane actions are
    /// registered on the workspace's element tree, so without this guard
    /// they would also move the mark while the main area is focused.
    fn panes_focused(&self, window: &Window) -> bool {
        self.pane_focus.clone().is_focused(window)
    }

    fn pane_up(&mut self, _: &PaneUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.panes_focused(window) {
            self.pane_move(-1, cx);
        }
    }

    fn pane_down(&mut self, _: &PaneDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.panes_focused(window) {
            self.pane_move(1, cx);
        }
    }

    fn pane_on_enter(&mut self, _: &PaneActivate, window: &mut Window, cx: &mut Context<Self>) {
        if self.panes_focused(window) {
            self.pane_activate(window, cx);
        }
    }

    fn pane_on_escape(&mut self, _: &PaneLeaveFocus, window: &mut Window, cx: &mut Context<Self>) {
        if self.panes_focused(window) {
            self.pane_leave_focus(window, cx);
        }
    }

    /// How many rows the visible pane has, for clamping the arrow keys.
    pub(crate) fn pane_row_count(&self, cx: &App) -> usize {
        match self.right_tab {
            RightTab::Tags => self.tag_rows(cx).len(),
            RightTab::Properties => self.property_rows(cx).len(),
            RightTab::Outline => self.outline_rows(cx).len(),
            // * Backlinks and Outgoing are card lists without a row model
            //   the arrows could walk; their rows stay click-only.
            _ => 0,
        }
    }

    /// The selection of the pane that has the keyboard, moved by `delta`
    /// rows and clamped to the pane's row count.
    pub(crate) fn pane_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.pane_row_count(cx);
        if count == 0 {
            return;
        }
        let selected = match self.right_tab {
            RightTab::Tags => &mut self.tag_state.selected,
            RightTab::Properties => &mut self.property_state.selected,
            RightTab::Outline => &mut self.outline_state.selected,
            _ => return,
        };
        let next = match selected {
            Some(ix) => (*ix as isize + delta).clamp(0, count as isize - 1) as usize,
            None => {
                if delta >= 0 {
                    0
                } else {
                    count - 1
                }
            }
        };
        *selected = Some(next);
        cx.notify();
    }

    /// Enter on the selected row: the same thing a click on it does — a tag
    /// or property row searches for it, an outline row reveals its heading
    /// in the note.
    pub(crate) fn pane_activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ix = match self.right_tab {
            RightTab::Tags => self.tag_state.selected,
            RightTab::Properties => self.property_state.selected,
            RightTab::Outline => self.outline_state.selected,
            _ => None,
        };
        let Some(ix) = ix else {
            return;
        };
        match self.right_tab {
            RightTab::Tags => {
                let rows = self.tag_rows(cx);
                if let Some(row) = rows.get(ix) {
                    let path = row.path.clone();
                    self.show_search(&format!("tag:#{path}"), Some(window), cx);
                }
            }
            RightTab::Properties => {
                let rows = self.property_rows(cx);
                if let Some(row) = rows.get(ix) {
                    let name = row.name.clone();
                    self.show_search(&format!("[\"{name}\"]"), Some(window), cx);
                }
            }
            RightTab::Outline => {
                let rows = self.outline_rows(cx);
                if let Some(row) = rows.get(ix) {
                    self.reveal_heading(row.offset, cx);
                }
            }
            _ => {}
        }
    }

    /// Escape hands the keyboard back to the main area and drops the mark.
    pub(crate) fn pane_leave_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.right_tab {
            RightTab::Tags => self.tag_state.selected = None,
            RightTab::Properties => self.property_state.selected = None,
            RightTab::Outline => self.outline_state.selected = None,
            _ => {}
        }
        self.focus_main(window, cx);
    }
}

/// Keys for the side panes, bound to the panes' own context beneath the
/// workspace's: a row click (or the dock tab) focuses the panes, the
/// arrows move the mark, Enter runs the row, Escape goes back to the note.
pub fn bind_pane_keys(cx: &mut App) {
    let ctx = Some("SidePane");
    cx.bind_keys([
        KeyBinding::new("up", PaneUp, ctx),
        KeyBinding::new("down", PaneDown, ctx),
        KeyBinding::new("enter", PaneActivate, ctx),
        KeyBinding::new("escape", PaneLeaveFocus, ctx),
    ]);
}
