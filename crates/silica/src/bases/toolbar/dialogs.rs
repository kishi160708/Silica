//! The Bases toolbar's dialog panels: the panels themselves, the views
//! and Configure view, and the results/sort/group/properties/filter views
//! — the methods of the split-out sections, unchanged apart from living in
//! their own module. Split from `bases/toolbar.rs` per the audit's
//! module-size finding.

use super::*;

impl BasesView {

    /// The open panel, anchored where its control was clicked.
    pub(crate) fn render_panel(
        &mut self,
        base: &Base,
        view: &View,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        self.toolbar.rows.clear();
        let body = match self.toolbar.panel {
            Panel::None => return None,
            Panel::Views => self.render_views_panel(base, theme, cx),
            Panel::Sort => self.render_sort_panel(base, view, theme, cx),
            Panel::Group => self.render_group_panel(base, view, theme, cx),
            Panel::Properties => self.render_properties_panel(base, view, theme, cx),
            Panel::Filter => self.render_filter_panel(base, view, theme, cx),
            Panel::Results => self.render_results_panel(view, theme, cx),
        };
        let at = self.toolbar.at;
        Some(
            deferred(
                anchored()
                    .position(at)
                    .anchor(Anchor::TopLeft)
                    .snap_to_window_with_margin(px(8.))
                    .child(body),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }

    /// The `.menu.bases-toolbar-menu` box every panel sits in.
    pub(crate) fn panel_box(
        &self,
        id: &'static str,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        menu(id, theme)
            .min_w(px(260.))
            .max_w(px(420.))
            .gap(px(2.))
            .on_mouse_down_out(cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                this.close_panel_out(event.position, cx)
            }))
    }

    /// Opens the views panel on `ix`'s **Configure view** page, which is
    /// what `bases:add-view` does after appending a view: "appends a view
    /// named `View 2` and opens its Configure view page" is the observed
    /// effect, and this is its second half.
    pub(crate) fn configure_view(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.toolbar.panel = Panel::Views;
        self.toolbar.configuring = Some(ix);
        self.toolbar.combo = None;
        cx.notify();
    }

    // ----- views + Configure view --------------------------------------------

    /// The views panel: one row per view, the row's own icon opening that
    /// view's Configure view page.
    pub(crate) fn render_views_panel(
        &mut self,
        base: &Base,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(ix) = self.toolbar.configuring {
            return self.render_configure(base, ix, theme, cx);
        }
        let views: Vec<View> = if base.views.is_empty() {
            vec![default_view()]
        } else {
            base.views.clone()
        };
        self.toolbar.rows = views
            .iter()
            .map(|v| json!({ "name": v.name, "type": v.kind }))
            .collect();
        let showing = self.view_ix();
        let mut body = self.panel_box("bases-toolbar-views-menu", theme, cx);
        for (ix, view) in views.iter().enumerate() {
            let icon_name = layout::kind_icon(&view.kind).unwrap_or("file-question");
            body = body.child(
                div()
                    .id(("bases-view-row", ix))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .when(ix == showing, |d| d.bg(theme.bg_active))
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.show_view(ix, cx);
                        this.close_panel(cx);
                    }))
                    .child(
                        // * The row's own icon is the Configure view page's
                        //   handle, which is observed.
                        div()
                            .id(("bases-view-configure", ix))
                            .size(px(20.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .hover(|s| s.bg(theme.bg_active))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.show_view(ix, cx);
                                this.toolbar.configuring = Some(ix);
                                cx.notify();
                            }))
                            .child(icon(icon_name, 14., theme.text_muted)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(SharedString::from(view.name.clone())),
                    ),
            );
        }
        body.into_any_element()
    }

    /// The Configure view page: the Layout combobox and that layout's own
    /// controls.
    ///
    /// Observed: the combobox offers exactly Table, Cards, List and Kanban,
    /// and picking one **rewrites `type` and leaves the previous layout's
    /// keys in place**. Obsidian 1.14.5 marks *Table* as the selected row
    /// whatever the view's real layout is; that is a bug the run recorded as
    /// one, so this marks the real layout instead.
    ///
    /// The page's name field is **not** drawn: what renaming a view through
    /// it writes was never observed, so the name shows as text.
    pub(crate) fn render_configure(
        &mut self,
        base: &Base,
        ix: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = base.views.get(ix).cloned().unwrap_or_else(default_view);
        let kind = view.kind.clone();
        let labels: Vec<SharedString> = LAYOUTS
            .iter()
            .map(|(_, label, _)| SharedString::from(*label))
            .collect();
        let selected = LAYOUTS.iter().position(|(k, ..)| *k == kind);
        let shown = layout::kind_label(&kind)
            .map(SharedString::from)
            .unwrap_or_else(|| SharedString::from(kind.clone()));
        let mut body = self
            .panel_box("bases-toolbar-configure", theme, cx)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .px(px(4.))
                    .pb(px(4.))
                    .child(
                        div()
                            .id("bases-configure-back")
                            .size(px(20.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_alt))
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.toolbar.configuring = None;
                                this.toolbar.combo = None;
                                this.toolbar.editing = None;
                                cx.notify();
                            }))
                            .child(icon("arrow-left", 14., theme.text_muted)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .text_color(theme.text)
                            .child(SharedString::from(view.name.clone())),
                    ),
            )
            .child(self.setting_row(
                "Layout",
                self.combobox("layout", 0, Some(shown), &labels, selected, theme, cx),
                theme,
            ));
        self.toolbar.rows = vec![json!({ "control": "Layout", "value": kind })];
        // ? Each layout's own keys, from the observed layout-key table. The
        //   label of every control but Row height is **unprobed** — the
        //   Configure view page's own wording was never read — so the rest show
        //   the key they write.
        match Layout::of(&view) {
            Layout::Table(table) => {
                let steps: Vec<SharedString> = RowHeight::all()
                    .iter()
                    .map(|h| SharedString::from(h.label()))
                    .collect();
                let now = RowHeight::all().iter().position(|h| *h == table.row_height);
                let shown = SharedString::from(table.row_height.label());
                self.record_control("rowHeight", table.row_height.key());
                body = body.child(self.setting_row(
                    "Row height",
                    self.combobox("rowHeight", 0, Some(shown), &steps, now, theme, cx),
                    theme,
                ));
            }
            Layout::Cards(cards) => {
                self.record_control("cardSize", &cards.card_size.to_string());
                body = body.child(self.setting_row(
                    "cardSize",
                    self.slider("cardSize", cards.card_size, CARD_SIZE_RANGE, theme, cx),
                    theme,
                ));
                body = body.child(self.image_row(base, &view, cards.image.as_deref(), theme, cx));
                body = body.child(self.image_fit_row(cards.image_fit, theme, cx));
                self.record_control("imageAspectRatio", &cards.image_aspect_ratio.to_string());
                body = body.child(self.setting_row(
                    "imageAspectRatio",
                    self.slider(
                        "imageAspectRatio",
                        cards.image_aspect_ratio,
                        ASPECT_RANGE,
                        theme,
                        cx,
                    ),
                    theme,
                ));
            }
            Layout::Kanban(kanban) => {
                self.record_control("columnWidth", &kanban.column_width.to_string());
                body = body.child(self.setting_row(
                    "columnWidth",
                    self.slider(
                        "columnWidth",
                        kanban.column_width,
                        COLUMN_WIDTH_RANGE,
                        theme,
                        cx,
                    ),
                    theme,
                ));
                body = body.child(self.image_row(base, &view, kanban.image.as_deref(), theme, cx));
                body = body.child(self.image_fit_row(kanban.image_fit, theme, cx));
                self.record_control("imageAspectRatio", &kanban.image_aspect_ratio.to_string());
                body = body.child(self.setting_row(
                    "imageAspectRatio",
                    self.slider(
                        "imageAspectRatio",
                        kanban.image_aspect_ratio,
                        ASPECT_RANGE,
                        theme,
                        cx,
                    ),
                    theme,
                ));
            }
            Layout::List(list) => {
                let marks: Vec<SharedString> = Markers::all()
                    .iter()
                    .map(|m| SharedString::from(m.label()))
                    .collect();
                let now = Markers::all().iter().position(|m| *m == list.markers);
                let shown = SharedString::from(list.markers.label());
                self.record_control("markers", list.markers.key());
                body = body.child(self.setting_row(
                    "markers",
                    self.combobox("markers", 0, Some(shown), &marks, now, theme, cx),
                    theme,
                ));
                self.record_control("separator", &list.separator);
                body = body.child(self.setting_row(
                    "separator",
                    self.text_field(Field::Separator, &list.separator, theme, cx),
                    theme,
                ));
                // ? "Indent properties" is not drawn: the key it writes is
                //   unprobed, and nothing should write `indentProperties`.
            }
            // * An unknown type never reaches the toolbar; the shell draws the
            //   error block instead.
            Layout::Unknown(_) => {}
        }
        body.into_any_element()
    }

    /// The `image` property picker, which writes a **qualified** id.
    pub(crate) fn image_row(
        &self,
        base: &Base,
        view: &View,
        current: Option<&str>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let properties = self.properties_of(base, view);
        // ? The picker's own "no property" row was never read; this is the one
        //   label in the module that is not observed, and it only clears a key.
        let mut labels = vec![SharedString::from("None")];
        labels.extend(properties.iter().map(|p| SharedString::from(p.1.clone())));
        let selected = match current {
            None => Some(0),
            Some(id) => properties
                .iter()
                .position(|p| same_property(&p.0, id))
                .map(|ix| ix + 1),
        };
        let shown = selected
            .and_then(|ix| labels.get(ix).cloned())
            .unwrap_or_else(|| SharedString::from(current.unwrap_or("None").to_string()));
        self.setting_row(
            "image",
            self.combobox("image", 0, Some(shown), &labels, selected, theme, cx),
            theme,
        )
    }

    pub(crate) fn image_fit_row(&self, fit: ImageFit, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let labels = [SharedString::from("Cover"), SharedString::from("Contain")];
        let now = match fit {
            ImageFit::Cover => 0,
            ImageFit::Contain => 1,
        };
        self.setting_row(
            "imageFit",
            self.combobox(
                "imageFit",
                0,
                Some(labels[now].clone()),
                &labels,
                Some(now),
                theme,
                cx,
            ),
            theme,
        )
    }

    pub(crate) fn record_control(&mut self, key: &str, value: &str) {
        self.toolbar
            .rows
            .push(json!({ "control": key, "value": value }));
    }

    // ----- results ------------------------------------------------------------

    /// The results panel: the limit input, *Show all (n)* and *Copy to
    /// clipboard*.
    ///
    /// Observed: the input writes `limit` as a bare number and *Show all*
    /// drops it; *Copy to clipboard* produces the same TSV `bases:copy-table`
    /// does. ***Export CSV...*** is not drawn — see the module note.
    pub(crate) fn render_results_panel(
        &mut self,
        view: &View,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let total = self.toolbar.total.unwrap_or(0);
        let limit = view.limit;
        self.toolbar.rows = vec![
            json!({ "control": "Limit number of results", "value": limit }),
            json!({ "row": format!("Show all ({total})") }),
            json!({ "row": "Copy to clipboard" }),
        ];
        let field = self.text_field(
            Field::Limit,
            &limit.map(|n| n.to_string()).unwrap_or_default(),
            theme,
            cx,
        );
        self.panel_box("bases-toolbar-results", theme, cx)
            .child(self.setting_row("Limit number of results", field, theme))
            .child(self.panel_row(
                "bases-results-show-all",
                0,
                None,
                SharedString::from(format!("Show all ({total})")),
                theme,
                cx.listener(|this, _, _, cx| {
                    cx.stop_propagation();
                    this.edit_view(|view| layout::set_limit(view, None), cx);
                    let total = this.unlimited_rows();
                    this.toolbar.total = Some(total);
                    // * The field has to follow the key it writes; the change
                    //   event that raises is a no-op, because `limit` is
                    //   already gone.
                    if let Some(input) = this.toolbar.field.clone() {
                        input.update(cx, |input, cx| input.set_text("", cx));
                    }
                    cx.notify();
                }),
            ))
            .child(self.panel_row(
                "bases-results-copy",
                0,
                Some("copy"),
                SharedString::from("Copy to clipboard"),
                theme,
                cx.listener(|this, _, _, cx| {
                    cx.stop_propagation();
                    this.copy_table(cx);
                }),
            ))
            .into_any_element()
    }

    /// The visible columns as TSV, on the clipboard, with Obsidian's own
    /// notice.
    ///
    /// Observed for `bases:copy-table`: one header row of display names, tab
    /// separators, no trailing newline, and a "Copied to your clipboard"
    /// notice. The results panel's own row produces the same TSV.
    pub(crate) fn copy_table(&mut self, cx: &mut Context<Self>) {
        let Some(table) = self.table() else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(table_tsv(table)));
        self.report("Copied to your clipboard", cx);
        self.close_panel(cx);
    }

    // ----- sort ---------------------------------------------------------------

    /// The sort panel: a (property, direction) pair per entry with a
    /// `lucide-trash-2`, over the row that adds one.
    ///
    /// Observed: an insertion into `sort` **prepends** — *Sort Z → A* puts
    /// `{property, direction: DESC}` at the head of the list. The panel's own
    /// add affordance was never read, so it is drawn at the head and
    /// prepends too, which is the only insertion the run pins down.
    pub(crate) fn render_sort_panel(
        &mut self,
        base: &Base,
        view: &View,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let properties = self.properties_of(base, view);
        let labels: Vec<SharedString> = properties
            .iter()
            .map(|p| SharedString::from(p.1.clone()))
            .collect();
        let directions: Vec<SharedString> = DIRECTIONS
            .iter()
            .map(|(_, label)| SharedString::from(*label))
            .collect();
        self.toolbar.rows = view
            .sort
            .iter()
            .map(|s| json!({ "property": s.property, "direction": s.direction }))
            .collect();
        let mut body = self
            .panel_box("bases-toolbar-sort", theme, cx)
            .child(self.add_row("sortAdd", &labels, theme, cx));
        for (ix, entry) in view.sort.iter().enumerate() {
            let selected = properties
                .iter()
                .position(|p| same_property(&p.0, &entry.property));
            let shown = selected
                .and_then(|i| labels.get(i).cloned())
                .unwrap_or_else(|| SharedString::from(entry.property.clone()));
            let down = direction_ix(entry.direction.as_deref());
            body = body.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .px(px(4.))
                    .child(div().flex_1().min_w(px(0.)).child(self.combobox(
                        "sortProperty",
                        ix,
                        Some(shown),
                        &labels,
                        selected,
                        theme,
                        cx,
                    )))
                    .child(self.combobox(
                        "sortDirection",
                        ix,
                        Some(directions[down].clone()),
                        &directions,
                        Some(down),
                        theme,
                        cx,
                    ))
                    .child(self.trash(
                        "bases-sort-trash",
                        ix,
                        theme,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.edit_view(move |view| remove_sort(view, ix), cx);
                        }),
                    )),
            );
        }
        body.into_any_element()
    }

    // ----- group --------------------------------------------------------------

    /// The group panel: the same (property, direction) pair for `groupBy`,
    /// which is one entry rather than a list.
    ///
    /// Nothing here writes or clears `groupOrder`: whether the UI ever writes
    /// one is unprobed, so a `groupOrder` the file carries is left exactly as
    /// it was found.
    pub(crate) fn render_group_panel(
        &mut self,
        base: &Base,
        view: &View,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let properties = self.properties_of(base, view);
        let labels: Vec<SharedString> = properties
            .iter()
            .map(|p| SharedString::from(p.1.clone()))
            .collect();
        let directions: Vec<SharedString> = DIRECTIONS
            .iter()
            .map(|(_, label)| SharedString::from(*label))
            .collect();
        self.toolbar.rows = view
            .group_by
            .iter()
            .map(|g| json!({ "property": g.property, "direction": g.direction }))
            .collect();
        let mut body = self.panel_box("bases-toolbar-group", theme, cx);
        match &view.group_by {
            None => body = body.child(self.add_row("groupAdd", &labels, theme, cx)),
            Some(group) => {
                let selected = properties
                    .iter()
                    .position(|p| same_property(&p.0, &group.property));
                let shown = selected
                    .and_then(|i| labels.get(i).cloned())
                    .unwrap_or_else(|| SharedString::from(group.property.clone()));
                let down = direction_ix(group.direction.as_deref());
                body = body.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.))
                        .px(px(4.))
                        .child(div().flex_1().min_w(px(0.)).child(self.combobox(
                            "groupProperty",
                            0,
                            Some(shown),
                            &labels,
                            selected,
                            theme,
                            cx,
                        )))
                        .child(self.combobox(
                            "groupDirection",
                            0,
                            Some(directions[down].clone()),
                            &directions,
                            Some(down),
                            theme,
                            cx,
                        ))
                        .child(self.trash(
                            "bases-group-trash",
                            0,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.edit_view(|view| view.group_by = None, cx);
                            }),
                        )),
                );
            }
        }
        body.into_any_element()
    }

    // ----- properties ---------------------------------------------------------

    /// The properties panel: a checkbox per property, over the field that
    /// narrows the list and the "Add *name* property" row.
    ///
    /// Observed: `order` is exactly the shown columns, so checking a property
    /// appends its id and unchecking removes it. A box is checked when the
    /// resolved table shows that column, which is also what makes unchecking
    /// work on a view with no `order` at all: the implied columns are written
    /// out minus the one unchecked. That materialization is a **reading** —
    /// no fixture reached the renderer without an `order`.
    ///
    /// "Hide all" and "Add formula" are not drawn; see the module note.
    pub(crate) fn render_properties_panel(
        &mut self,
        base: &Base,
        view: &View,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let properties = self.properties_of(base, view);
        let shown = self.shown_columns();
        let typed = match self.toolbar.editing {
            Some(Field::Property) => self.field_text(cx),
            _ => String::new(),
        };
        let needle = typed.trim().to_ascii_lowercase();
        let matching: Vec<(usize, (String, String))> = properties
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, (id, label))| {
                needle.is_empty()
                    || label.to_ascii_lowercase().contains(&needle)
                    || id.to_ascii_lowercase().contains(&needle)
            })
            .collect();
        self.toolbar.rows = matching
            .iter()
            .map(|(_, (id, label))| {
                json!({
                    "id": id,
                    "label": label,
                    "checked": shown.iter().any(|s| same_property(s, id)),
                })
            })
            .collect();
        let mut body = self.panel_box("bases-toolbar-properties", theme, cx).child(
            div()
                .px(px(4.))
                .pb(px(4.))
                .child(self.text_field(Field::Property, &typed, theme, cx)),
        );
        for (ix, (id, label)) in matching {
            let checked = shown.iter().any(|s| same_property(s, &id));
            let write = id.clone();
            let shown_now = shown.clone();
            body = body.child(
                div()
                    .id(("bases-property-row", ix))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        let id = write.clone();
                        let shown = shown_now.clone();
                        this.edit_view(move |view| toggle_property(view, &shown, &id), cx);
                    }))
                    .child(check_box(checked, theme))
                    .child(
                        div()
                            .flex_1()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(SharedString::from(label)),
                    ),
            );
        }
        // * "Add *name* property": the italic *name* is the field's own text,
        //   which is how the row names a property the vault does not carry yet.
        let name = typed.trim().to_string();
        if !name.is_empty() && !properties.iter().any(|(_, label)| *label == name) {
            let id = layout::qualify(&name);
            body = body.child(self.panel_row(
                "bases-property-add",
                0,
                Some("plus"),
                SharedString::from(format!("Add {name} property")),
                theme,
                cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    let id = id.clone();
                    this.edit_view(move |view| add_property(view, &layout::qualify(&id)), cx);
                }),
            ));
        }
        body.into_any_element()
    }

    /// The ids of the columns the showing view resolves to, which is what the
    /// checkbox list reflects.
    pub(crate) fn shown_columns(&self) -> Vec<String> {
        self.table()
            .map(|t| t.columns.iter().map(|c| c.id.clone()).collect())
            .unwrap_or_default()
    }

    // ----- filter -------------------------------------------------------------

    /// The filter panel: the base's own `filters` under "All views" and the
    /// view's under "This view", a conjunction select per group and a
    /// statement field per rule.
    ///
    /// The **operator combobox is not drawn**, and with it the structured
    /// (property, operator, value) triple: the run read the rule's shape but
    /// never its operator list, so building one would invent the statement
    /// text a rule writes. The panel is therefore the expression mode the
    /// `filer-expression-mode-toggle` switches into — the toggle is drawn on
    /// each rule, lit, and says so — and a rule is its statement verbatim,
    /// which is the one form `silica_formats` round-trips byte for byte.
    ///
    /// An absent `filters` and a bare top-level statement are both shown as
    /// an `and` group, so every section has one group at its root; the first
    /// edit of a bare statement therefore writes it as `and: [..]`. What
    /// Obsidian writes for a single rule is unobserved.
    pub(crate) fn render_filter_panel(
        &mut self,
        _base: &Base,
        _view: &View,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let nodes = self.filter_nodes();
        self.toolbar.rows = nodes
            .iter()
            .map(|n| match &n.kind {
                NodeKind::Group(conj) => json!({
                    "section": n.section.as_str(),
                    "depth": n.depth,
                    "conjunction": conj,
                }),
                NodeKind::Rule(text) => json!({
                    "section": n.section.as_str(),
                    "depth": n.depth,
                    "rule": text,
                }),
            })
            .collect();
        let conjunctions: Vec<SharedString> = CONJUNCTIONS
            .iter()
            .map(|c| SharedString::from(*c))
            .collect();
        let mut body = self.panel_box("bases-toolbar-filter", theme, cx);
        let mut section = None;
        for (ix, node) in nodes.iter().enumerate() {
            if section != Some(node.section) {
                section = Some(node.section);
                body = body.child(
                    div()
                        .px(px(8.))
                        .pt(px(6.))
                        .pb(px(2.))
                        .text_size(px(FONT_UI_SMALLER))
                        .text_color(theme.text_faint)
                        .child(SharedString::from(node.section.heading())),
                );
            }
            let indent = px(4. + 12. * node.depth as f32);
            match &node.kind {
                NodeKind::Group(conj) => {
                    let now = CONJUNCTIONS.iter().position(|c| c == conj);
                    let shown = SharedString::from(*conj);
                    body = body.child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.))
                            .pl(indent)
                            .pr(px(4.))
                            .child(self.combobox(
                                "conjunction",
                                ix,
                                Some(shown),
                                &conjunctions,
                                now,
                                theme,
                                cx,
                            )),
                    );
                    // * The group's own new-rule field: typing in it appends a
                    //   rule to this group.
                    body = body.child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.))
                            .pl(px(4. + 12. * (node.depth + 1) as f32))
                            .pr(px(4.))
                            .child(div().flex_1().min_w(px(0.)).child(self.text_field(
                                Field::Rule(ix),
                                "",
                                theme,
                                cx,
                            ))),
                    );
                }
                NodeKind::Rule(text) => {
                    body = body.child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.))
                            .pl(indent)
                            .pr(px(4.))
                            .child(div().flex_1().min_w(px(0.)).child(self.text_field(
                                Field::Rule(ix),
                                text,
                                theme,
                                cx,
                            )))
                            .child(
                                // ? `filer-expression-mode-toggle`, spelled as
                                //   Obsidian spells it. It is lit and inert:
                                //   expression mode is the only mode this panel
                                //   has, because the operator list is unprobed.
                                div()
                                    .id(("bases-rule-mode", ix))
                                    .size(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .bg(theme.bg_active)
                                    .tooltip(tooltip(EXPRESSION_MODE_TOGGLE))
                                    .child(icon("code-xml", 14., theme.accent)),
                            )
                            .child(self.trash(
                                "bases-rule-trash",
                                ix,
                                theme,
                                cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.remove_rule_at(ix, cx);
                                }),
                            )),
                    );
                }
            }
        }
        body.into_any_element()
    }

    /// Every group and rule of both sections, flattened, which is what the
    /// filter panel's combobox and field keys index into.
    pub(crate) fn filter_nodes(&self) -> Vec<Node> {
        let Some(base) = self.base() else {
            return Vec::new();
        };
        let view = self.showing_view(base);
        let mut out = Vec::new();
        flatten(
            Section::AllViews,
            &group_tree(base.filters.as_ref()),
            Vec::new(),
            0,
            &mut out,
        );
        flatten(
            Section::ThisView,
            &group_tree(view.filters.as_ref()),
            Vec::new(),
            0,
            &mut out,
        );
        out
    }

    pub(crate) fn remove_rule_at(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(node) = self.filter_nodes().get(ix).cloned() else {
            return;
        };
        self.toolbar.editing = None;
        let path = node.path.clone();
        self.edit_filter(
            node.section,
            move |filter| {
                let mut tree = group_tree(filter.as_ref());
                remove_at(&mut tree, &path);
                *filter = written(tree);
            },
            cx,
        );
        cx.notify();
    }

    /// Routes a filter write to the section it belongs to: the base's own
    /// `filters` ("All views") or the showing view's ("This view").
    pub(crate) fn edit_filter<F>(&mut self, section: Section, edit: F, cx: &mut Context<Self>)
    where
        F: FnOnce(&mut Option<Filter>),
    {
        match section {
            Section::AllViews => self.edit_base(move |base| edit(&mut base.filters), cx),
            Section::ThisView => self.edit_view(move |view| edit(&mut view.filters), cx),
        }
    }
}
