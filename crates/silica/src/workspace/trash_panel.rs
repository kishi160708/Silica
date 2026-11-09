//! The Trash pane: what the vault's own `.trash/` holds, one row per
//! trashed entry, with the two things that can be done to one on its row
//! -- restore, and delete permanently.
//!
//! The moves themselves are the vault's ([`Vault::list_trash`],
//! [`Vault::restore_from_trash`], through `VaultState`): this is the panel
//! over them and the one thing the vault does not do, the permanent
//! delete, which leaves no record behind to keep.

use super::*;

/// A pane with nothing in it: Obsidian's `.pane-empty`, 13 px faint, 24 px
/// over and 16 px under, 8 px above and below.
fn empty_pane(text: &str, theme: &Theme) -> gpui::AnyElement {
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
                .child(text.to_string()),
        )
        .into_any_element()
}

impl Workspace {
    /// Reads `.trash/` into the pane's own state. Called when the pane is
    /// shown and after a row's button has run, so an entry trashed from
    /// outside the pane -- the explorer's delete -- is in it too.
    pub(crate) fn load_trash_entries(&mut self, cx: &mut Context<Self>) {
        let read = self.state.as_ref().map(|state| state.list_trash());
        match read {
            Some(Ok(entries)) => {
                self.trash_state.entries = entries;
                self.trash_state.error = None;
            }
            // * An unreadable folder is not an empty one: the pane says
            //   what went wrong instead of showing a list that is short.
            Some(Err(err)) => {
                self.trash_state.entries.clear();
                self.trash_state.error = Some(err);
            }
            None => {
                self.trash_state.entries.clear();
                self.trash_state.error = None;
            }
        }
        // * A row that is gone cannot keep the mark, and a list that got
        //   shorter must not leave it past the end.
        let rows = self.trash_state.entries.len();
        self.trash_state.selected = self.trash_state.selected.filter(|&ix| ix < rows);
        cx.notify();
    }

    /// `silica:workspace-show-trash`: the Trash pane in the right dock,
    /// beside the note, with the folder read now rather than whenever the
    /// pane last drew.
    pub(crate) fn show_trash_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.right_visible = true;
        self.show_right_tab("trash", cx);
        self.load_trash_entries(cx);
        // * The dock tab takes the keyboard the way a row click does, so
        //   the arrows walk the rows from the start.
        self.focus_panes(window, cx);
    }

    /// A row's restore button: back to the folder the entry was trashed
    /// from, recreating the parents that went with it, and back into the
    /// index so the note is searchable and linked the moment it lands.
    pub(crate) fn restore_trash_entry(&mut self, name: String, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            self.report("no vault loaded", cx);
            return;
        };
        match state.restore_from_trash(&name) {
            Ok(changes) => {
                self.load_trash_entries(cx);
                self.after_changes(changes, cx);
            }
            // * A name the vault already holds is refused rather than
            //   traded for the older copy, which is the only way a restore
            //   can fail: the row stays, and the pane says why.
            Err(err) => {
                self.trash_state.error = Some(err.clone());
                self.report(err, cx);
                cx.notify();
            }
        }
    }

    /// A row's delete button: the entry leaves `.trash/` for good. Nothing
    /// records it after that, so there is no undo -- which is what the
    /// button says it is doing.
    pub(crate) fn delete_trash_entry(&mut self, name: String, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            self.report("no vault loaded", cx);
            return;
        };
        match state.delete_from_trash(&name) {
            Ok(()) => {
                self.load_trash_entries(cx);
                self.report(format!("Deleted {name} permanently"), cx);
            }
            Err(err) => {
                self.trash_state.error = Some(err.clone());
                self.report(err, cx);
                cx.notify();
            }
        }
    }

    /// The Trash pane: a re-read of the folder in the header, then one row
    /// per entry. A row's own click only marks it, so the arrow keys and
    /// Enter have something to work on; the two buttons on the row do the
    /// work, because one of them cannot be undone.
    pub(crate) fn render_trash_pane(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let header = Self::pane_header().child(
            Self::nav_button(
                "trash-reread",
                "rotate-ccw",
                "Re-read the trash folder",
                false,
                theme,
            )
            .on_click(cx.listener(|this, _, _, cx| this.load_trash_entries(cx))),
        );
        let selected = self.trash_state.selected;
        let body: gpui::AnyElement = if let Some(err) = self.trash_state.error.clone() {
            empty_pane(&err, theme)
        } else if self.trash_state.entries.is_empty() {
            empty_pane("No files in the trash.", theme)
        } else {
            let mut list = div()
                .id("trash")
                .flex()
                .flex_col()
                .flex_1()
                .overflow_y_scroll()
                .px(px(12.))
                .pt(px(4.))
                .pb(px(32.));
            for (ix, name) in self.trash_state.entries.clone().into_iter().enumerate() {
                let active = selected == Some(ix);
                let kind_icon = match FileKind::from_path(&name) {
                    FileKind::Image => "image",
                    FileKind::Markdown | FileKind::Canvas | FileKind::Base | FileKind::Pdf => {
                        "file-text"
                    }
                    _ => "file",
                };
                let restore_name = name.clone();
                let delete_name = name.clone();
                let row = div()
                    .id(("trash-row", ix))
                    .group("trash-row")
                    .w_full()
                    .flex_shrink_0()
                    .min_h(px(25.))
                    .mb(px(2.))
                    .px(px(8.))
                    .py(px(4.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .rounded(px(8.))
                    .cursor_pointer()
                    .when(active, |d| d.bg(theme.bg_alt))
                    .hover(|s| s.bg(theme.bg_alt))
                    .child(
                        div()
                            .w(px(16.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(theme.text_muted)
                            .child(icon(kind_icon, 14., theme.text_muted)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(FONT_UI_SMALL))
                            .line_height(px(17.))
                            .text_color(if active { theme.text } else { theme.text_muted })
                            .child(
                                div()
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(name),
                            ),
                    )
                    // * The two buttons: the first movable thing on the
                    //   row, the last one-way. Each stops the click, so
                    //   pressing one never marks the row as well.
                    .child(Self::trash_button(
                        ("trash-restore", ix),
                        "rotate-ccw",
                        "Restore to its original folder",
                        theme,
                        cx,
                        move |this, cx| this.restore_trash_entry(restore_name.clone(), cx),
                    ))
                    .child(Self::trash_button(
                        ("trash-delete", ix),
                        "trash-2",
                        "Delete permanently",
                        theme,
                        cx,
                        move |this, cx| this.delete_trash_entry(delete_name.clone(), cx),
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.trash_state.selected = Some(ix);
                        cx.notify();
                    }));
                list = list.child(row);
            }
            list.into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(header)
            .child(body)
            .into_any_element()
    }

    /// One of a Trash row's two buttons: 20 by 20, muted until the pointer
    /// is over the row, and a tooltip for what it does -- the delete
    /// above all, since it is the one that cannot be taken back.
    fn trash_button(
        id: (&'static str, usize),
        icon_name: &'static str,
        label: &'static str,
        theme: &Theme,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex_shrink_0()
            .w(px(22.))
            .h(px(22.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .rounded(px(4.))
            .text_color(theme.text_muted)
            .group_hover("trash-row", |s| s.text_color(theme.text))
            .tooltip(tooltip(label))
            .child(icon(icon_name, 14., theme.text_muted))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                on_click(this, cx);
            }))
    }
}
