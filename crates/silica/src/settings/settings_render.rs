//! The Settings overlay's paint code: the window frame, tab strip, rows,
//! and the controls each row draws — the methods of the split-out
//! rendering section, unchanged apart from living in their own module.
//! Split from `settings.rs` per the audit's module-size finding.

use super::*;

impl SettingsView {

    pub(crate) fn render_nav(&self, theme: &Theme, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let group_title = |label: &'static str, theme: &Theme| {
            div()
                .px_3()
                .pt_3()
                .pb_1()
                .text_size(px(11.))
                .text_color(theme.text_faint)
                .child(label.to_uppercase())
        };
        let mut nav = div()
            .id("settings-nav")
            .flex()
            .flex_col()
            // * A short window clips the tabs below Core plugins, and the
            //   root is `overflow_hidden`: the nav scrolls for the same
            //   reason the content list does.
            .overflow_y_scroll()
            .w(px(220.))
            .h_full()
            .flex_shrink_0()
            .bg(theme.bg_secondary)
            .border_r_1()
            .border_color(theme.border)
            .py_2()
            // * Obsidian's sidebar opens straight on "Options"; the mark and
            //   the version above it are ours, and say which program wrote
            //   the `.obsidian/` the rest of this modal is editing.
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .pt_1()
                    .pb_2()
                    .child(logo(22.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.text)
                                    .child("Silica"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme.text_faint)
                                    .child(concat!("v", env!("CARGO_PKG_VERSION"))),
                            ),
                    ),
            )
            .child(group_title(crate::i18n::t("Options"), theme));
        for tab in Tab::OPTIONS {
            nav = nav.child(self.nav_item(*tab, theme, cx));
        }
        let plugin_tabs: Vec<&'static str> = match &self.state {
            Some(state) => PLUGIN_TABS
                .iter()
                .filter(|(id, _)| {
                    CORE_PLUGINS.iter().find(|p| p.id == *id).is_some_and(|p| {
                        CorePlugins::is_enabled_or_default(state.core_plugins().as_ref(), p)
                    })
                })
                .map(|(id, _)| *id)
                .collect(),
            None => Vec::new(),
        };
        if !plugin_tabs.is_empty() {
            nav = nav.child(group_title("Core plugins", theme));
            for id in plugin_tabs {
                nav = nav.child(self.nav_item(Tab::Plugin(id), theme, cx));
            }
        }
        nav
    }

    pub(crate) fn nav_item(
        &self,
        tab: Tab,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let active = self.tab == tab;
        div()
            .id(SharedString::from(format!("settings-nav-{}", tab.name())))
            .mx_2()
            .px_2()
            .py(px(5.))
            .rounded_sm()
            .cursor_pointer()
            .text_size(px(13.))
            .text_color(if active { theme.text } else { theme.text_muted })
            .when(active, |d| d.bg(theme.bg_active))
            .hover(|s| s.bg(theme.bg_alt))
            .on_click(cx.listener(move |this, _, window, cx| {
                if this.click_dismissed_a_popover(cx) {
                    return;
                }
                this.select_tab(tab, window, cx)
            }))
            .child(crate::i18n::t(tab.name()))
    }

    pub(crate) fn render_control(
        &self,
        item: &Item,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match &item.control {
            Control::None => div().into_any_element(),
            Control::Toggle(on) => {
                let on = *on;
                let toggle = div()
                    .relative()
                    .w(px(36.))
                    .h(px(20.))
                    .rounded_full()
                    .bg(if on { theme.accent } else { theme.bg_active })
                    .border_1()
                    .border_color(if on { theme.accent } else { theme.border })
                    .child(
                        div()
                            .absolute()
                            .top(px(2.))
                            .left(px(if on { 17. } else { 2. }))
                            .size(px(14.))
                            .rounded_full()
                            .bg(rgb(0xffffff)),
                    );
                let Some(flag) = item.flag.clone() else {
                    return toggle.into_any_element();
                };
                div()
                    .id(SharedString::from(format!("settings-toggle-{}", item.name)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if this.click_dismissed_a_popover(cx) {
                            return;
                        }
                        this.toggle_flag(flag.clone(), window, cx)
                    }))
                    .child(toggle)
                    .into_any_element()
            }
            Control::Dropdown(value) => {
                let shown = div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py(px(3.))
                    .rounded_sm()
                    .bg(theme.bg_alt)
                    .border_1()
                    .border_color(theme.border)
                    .text_size(px(12.))
                    .text_color(theme.text)
                    .child(value.clone())
                    .child(div().text_color(theme.text_muted).child("▾"));
                let Some(typed) = item.typed else {
                    return shown.into_any_element();
                };
                let selected = self.state.as_ref().map(|s| typed.selected(s)).unwrap_or(0);
                let open = self.menu == Some(typed.key);
                div()
                    .relative()
                    .child(
                        div()
                            .id(SharedString::from(format!("settings-menu-{}", typed.key)))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                // * The root closes whatever is open on any
                                //   click; this one has to survive it, or the
                                //   menu would shut again as the click bubbles.
                                cx.stop_propagation();
                                // * A click with a popover open is spent
                                //   closing it — this one's own second click
                                //   included, which is how the property type
                                //   menu closes too.
                                if this.click_dismissed_a_popover(cx) {
                                    return;
                                }
                                this.commit_field(cx);
                                this.menu = Some(typed.key);
                                cx.notify();
                            }))
                            .child(shown),
                    )
                    .when(open, |d| {
                        // * Deferred, or the rows after this one paint over
                        //   the options: a later sibling wins otherwise.
                        d.child(deferred(
                            div()
                                .absolute()
                                .top(px(26.))
                                .right(px(0.))
                                .min_w(px(200.))
                                .flex()
                                .flex_col()
                                .bg(theme.bg_secondary)
                                .border_1()
                                .border_color(theme.border)
                                .rounded_md()
                                .shadow_lg()
                                .py_1()
                                .children(typed.options.iter().enumerate().map(
                                    |(ix, (label, _))| {
                                        div()
                                            .id(SharedString::from(format!(
                                                "settings-option-{}-{ix}",
                                                typed.key
                                            )))
                                            .px_3()
                                            .py_1()
                                            .text_size(px(12.))
                                            .cursor_pointer()
                                            .when(ix == selected, |d| d.bg(theme.bg_active))
                                            .hover(|s| s.bg(theme.bg_alt))
                                            .text_color(theme.text)
                                            .child(SharedString::from(label.to_string()))
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                cx.stop_propagation();
                                                this.pick(typed, ix, window, cx)
                                            }))
                                    },
                                )),
                        ))
                    })
                    .into_any_element()
            }
            Control::Text(_)
                if item
                    .field
                    .zip(self.editing)
                    .is_some_and(|(a, b)| a.same(&b)) =>
            {
                div()
                    .id("settings-field-open")
                    .min_w(px(220.))
                    .max_w(px(320.))
                    // * A click inside the open input is not a click away from
                    //   it, which is what the root would read it as.
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(self.field.clone())
                    .into_any_element()
            }
            Control::Text(value) if item.field.is_some() => {
                let field = item.field.expect("checked above");
                div()
                    .id(SharedString::from(format!("settings-field-{}", field.key)))
                    .min_w(px(160.))
                    .max_w(px(320.))
                    .px_2()
                    .py(px(3.))
                    .rounded_sm()
                    .bg(theme.bg_alt)
                    .border_1()
                    .border_color(theme.border)
                    .text_size(px(12.))
                    .cursor_text()
                    .text_color(if value.is_empty() {
                        theme.text_faint
                    } else {
                        theme.text
                    })
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(if value.is_empty() {
                        SharedString::from(field.placeholder)
                    } else {
                        value.clone()
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        // * Without this the click would bubble to the root and
                        //   be read as a click away from the field it just
                        //   opened, closing it again on the same frame.
                        cx.stop_propagation();
                        if this.click_dismissed_a_popover(cx) {
                            return;
                        }
                        this.edit_field(field, window, cx)
                    }))
                    .into_any_element()
            }
            Control::Text(value) => div()
                .min_w(px(160.))
                .max_w(px(320.))
                .px_2()
                .py(px(3.))
                .rounded_sm()
                .bg(theme.bg_alt)
                .border_1()
                .border_color(theme.border)
                .text_size(px(12.))
                .text_color(if value.is_empty() {
                    theme.text_faint
                } else {
                    theme.text
                })
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(if value.is_empty() {
                    SharedString::from("Empty")
                } else {
                    value.clone()
                })
                .into_any_element(),
            Control::Color(value) => {
                let stored = value.clone();
                let swatch = Hsv::of_hex(&stored)
                    .map(|h| h.hsla())
                    .unwrap_or(theme.accent);
                let at_default = stored.is_empty();
                let open = self.menu == Some(ACCENT_COLOR.key);
                let popover = open.then(|| self.render_picker(theme, cx));
                div()
                    .relative()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .children(popover)
                    .child(
                        div()
                            .id("settings-accent-reset")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(24.))
                            .rounded_sm()
                            .text_color(if at_default {
                                theme.text_faint
                            } else {
                                theme.accent
                            })
                            .when(!at_default, |d| {
                                d.cursor_pointer().hover(|s| s.bg(theme.bg_alt))
                            })
                            .child(crate::assets::icon("rotate-ccw", 16., theme.accent))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                if this.click_dismissed_a_popover(cx) {
                                    return;
                                }
                                this.reset_accent(window, cx)
                            })),
                    )
                    .child(
                        div()
                            .id("settings-accent-swatch")
                            .size(px(22.))
                            .rounded_full()
                            .bg(swatch)
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                if this.click_dismissed_a_popover(cx) {
                                    return;
                                }
                                this.open_picker(window, cx)
                            })),
                    )
                    .into_any_element()
            }
            Control::Slider(value) => {
                let Some(number) = item.number else {
                    return div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(120.))
                                .h(px(4.))
                                .rounded_full()
                                .bg(theme.bg_active),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.text_muted)
                                .child(value.clone()),
                        )
                        .into_any_element();
                };
                let now = self.state.as_ref().map(|s| (number.read)(s)).unwrap_or(0);
                // * One cell per step: the ranges are small, so a click lands
                //   on a value without measuring the track.
                let steps = (number.min..=number.max).collect::<Vec<_>>();
                let width = (120. / steps.len() as f32).max(3.);
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(div().flex().flex_row().items_center().h(px(14.)).children(
                        steps.into_iter().map(|step| {
                            div()
                                .id(SharedString::from(format!(
                                    "settings-step-{}-{step}",
                                    number.key
                                )))
                                .w(px(width))
                                .h(px(4.))
                                .cursor_pointer()
                                .bg(if step <= now {
                                    theme.accent
                                } else {
                                    theme.bg_active
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    if this.click_dismissed_a_popover(cx) {
                                        return;
                                    }
                                    this.set_number(number, step, window, cx)
                                }))
                        }),
                    ))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.text_muted)
                            .child(value.clone()),
                    )
                    .into_any_element()
            }
            Control::Button(label) => {
                let label = label.clone();
                div()
                    .px_3()
                    .py(px(3.))
                    .rounded_sm()
                    .bg(theme.bg_alt)
                    .border_1()
                    .border_color(theme.border)
                    .text_size(px(12.))
                    .text_color(theme.text_muted)
                    .child(label.clone())
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _, _, cx| {
                        cx.emit(SettingsEvent::ButtonClicked(label.clone()));
                    }))
                    .into_any_element()
            }
        }
    }

    /// Opens the dropdown that writes `key`, for `--menu`: clicking one is
    /// what does it in the app.
    pub fn open_menu(&mut self, key: &str, cx: &mut Context<Self>) {
        self.menu = TYPED_ALL.iter().find(|t| t.key == key).map(|t| t.key);
        cx.notify();
    }

    /// Writes a slider's value, which the control also reads through.
    /// Opens a text field for typing, with what the vault holds in it.
    pub(crate) fn edit_field(&mut self, field: TextField, window: &mut Window, cx: &mut Context<Self>) {
        // * One input serves every row, so opening the next one is the last
        //   moment the one before it can be written.
        self.commit_field(cx);
        let Some(state) = self.state.clone() else {
            return;
        };
        let value = (field.read)(&state);
        self.editing = Some(field);
        self.menu = None;
        self.field.update(cx, |input, cx| {
            input.set_placeholder(field.placeholder);
            input.set_text(value, cx);
        });
        window.focus(&self.field.focus_handle(cx), cx);
        cx.notify();
    }

    /// Writes what was typed, which the row then reads back.
    pub(crate) fn commit_field(&mut self, cx: &mut Context<Self>) {
        let Some(field) = self.editing.take() else {
            return;
        };
        let Some(state) = self.state.clone() else {
            return;
        };
        let typed = self.field.read(cx).text().to_string();
        if (field.read)(&state) != typed {
            cx.emit(SettingsEvent::Changed(Box::new((field.write)(
                &state, &typed,
            ))));
        }
        cx.notify();
    }

    pub(crate) fn set_number(&mut self, number: Number, value: i64, window: &Window, cx: &mut Context<Self>) {
        self.after_open_field(window, cx, move |this, cx| {
            let Some(state) = this.state.clone() else {
                return;
            };
            let value = value.clamp(number.min, number.max);
            if (number.read)(&state) != value {
                cx.emit(SettingsEvent::Changed(Box::new((number.write)(
                    &state, value,
                ))));
            }
            cx.notify();
        });
    }

    /// Writes the option a dropdown's row names. The value comes from the
    /// same table the control reads through, so the two cannot drift.
    pub(crate) fn pick(&mut self, typed: Typed, option: usize, window: &Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.after_open_field(window, cx, move |this, cx| {
            let Some(state) = this.state.clone() else {
                return;
            };
            let Some((_, value)) = typed.options.get(option) else {
                return;
            };
            if (typed.read)(&state) != *value {
                cx.emit(SettingsEvent::Changed(Box::new((typed.write)(
                    &state, value,
                ))));
            }
            cx.notify();
        });
    }

    pub(crate) fn render_item(&self, item: &Item, theme: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        if item.heading {
            return div()
                .pt_5()
                .pb_1()
                .text_size(px(15.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text)
                .child(item.name.clone());
        }
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_6()
            .py(px(10.))
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .text_color(theme.text)
                                    .child(item.name.clone()),
                            )
                            .children(item.honored.tag().map(|tag| {
                                div()
                                    .flex_shrink_0()
                                    .px(px(5.))
                                    .py(px(1.))
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_size(px(10.))
                                    .text_color(theme.text_faint)
                                    .child(tag)
                            })),
                    )
                    .when(!item.desc.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.text_muted)
                                .child(item.desc.clone()),
                        )
                    }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .child(self.render_control(item, theme, cx)),
            )
    }

    pub(crate) fn render_hotkey_rows(
        &mut self,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Div> {
        let theme = cx.global::<Theme>().clone();
        let counts = self.chord_counts();
        let danger = theme.callout_color("danger");
        range
            .filter_map(|ix| {
                let c = self.all_commands.get(*self.hotkey_rows.get(ix)?)?;
                let capturing = self.capturing.as_deref() == Some(c.id.as_str());
                let id = c.id.clone();
                let chips: Vec<gpui::AnyElement> = c
                    .hotkeys
                    .iter()
                    .enumerate()
                    .map(|(hix, h)| {
                        let duplicate = keystroke(h)
                            .and_then(|k| counts.get(&k))
                            .is_some_and(|n| *n > 1);
                        let mut chip = hotkey_chip(hotkey_label(h), &theme)
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_1();
                        if duplicate {
                            chip = chip.border_color(danger).text_color(danger);
                        } else if c.custom {
                            chip = chip.border_color(theme.accent).text_color(theme.text);
                        }
                        let id = id.clone();
                        let hotkey = h.clone();
                        chip.child(
                            div()
                                .id(SharedString::from(format!("hotkey-remove-{}-{hix}", c.id)))
                                .cursor_pointer()
                                .text_color(theme.text_faint)
                                .hover(|s| s.text_color(theme.text))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.remove_hotkey(&id, hotkey.clone(), cx)
                                }))
                                .child("×"),
                        )
                        .into_any_element()
                    })
                    .collect();
                let icon = |name: String, glyph: &'static str, theme: &Theme| {
                    div()
                        .id(SharedString::from(name))
                        .w(px(22.))
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_size(px(14.))
                        .text_color(theme.text_muted)
                        .hover(|s| s.bg(theme.bg_alt).text_color(theme.text))
                        .child(glyph)
                };
                let add = {
                    let id = c.id.clone();
                    icon(format!("hotkey-add-{}", c.id), "⊕", &theme).on_click(cx.listener(
                        move |this, _, window, cx| this.start_capture(id.clone(), window, cx),
                    ))
                };
                let restore = c.custom.then(|| {
                    let id = c.id.clone();
                    icon(format!("hotkey-restore-{}", c.id), "↺", &theme).on_click(
                        cx.listener(move |this, _, _, cx| this.restore_default_hotkeys(&id, cx)),
                    )
                });
                Some(
                    div()
                        .w_full()
                        .h(px(36.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.text)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(SharedString::from(c.name.clone())),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .flex_shrink_0()
                                .gap_1()
                                .when(chips.is_empty() && !capturing, |d| {
                                    d.child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.text_faint)
                                            .child("Blank"),
                                    )
                                })
                                .children(chips)
                                .when(capturing, |d| {
                                    d.child(
                                        hotkey_chip("Press hotkey...".into(), &theme)
                                            .border_color(theme.accent)
                                            .text_color(theme.text),
                                    )
                                })
                                .children(restore)
                                .child(add),
                        ),
                )
            })
            .collect()
    }

    pub(crate) fn render_hotkeys(&mut self, theme: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let filter = self.hotkey_filter;
        let count = self.hotkey_rows.len();
        let mut filters = div().flex().flex_row().gap_1().py_2();
        for f in HotkeyFilter::ALL {
            let f = *f;
            let active = f == filter;
            filters = filters.child(
                div()
                    .id(SharedString::from(format!("hotkey-filter-{}", f.label())))
                    .px_2()
                    .py(px(2.))
                    .rounded_sm()
                    .cursor_pointer()
                    .text_size(px(11.))
                    .text_color(if active { theme.text } else { theme.text_muted })
                    .when(active, |d| d.bg(theme.bg_active))
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.hotkey_filter = f;
                        this.refresh_hotkeys(cx);
                    }))
                    .child(f.label()),
            );
        }
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.hotkey_search.clone())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(filters)
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .child(SharedString::from(format!("{count} commands"))),
                    ),
            )
            .child(
                uniform_list("hotkeys", count, cx.processor(Self::render_hotkey_rows))
                    .track_scroll(&self.hotkey_scroll)
                    .flex_1(),
            )
    }
}
