use std::collections::HashSet;

use imgui::{self as ig, im_str};

pub(crate) struct TabInfo<'a> {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) closable: bool,
    pub(crate) render: Box<dyn FnMut(&str) + 'a>,
}

#[derive(Debug, Clone)]
pub(crate) struct TabResult {
    pub(crate) selected_tab_index: Option<usize>,
    pub(crate) closed_tab_index: Option<usize>,
}

#[derive(Debug, Default)]
pub(crate) struct Tabs {
    rendered: bool,
    selected_tab_index: Option<usize>,
    selected_tab_id: Option<String>,
    windowed_tabs: HashSet<String>,
}

impl Tabs {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn render(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        tabs: &mut [TabInfo<'_>],
        open_tab_index: Option<usize>,
        allow_windowing: bool,
    ) -> TabResult {
        let id_token = ui.push_id(id);

        //root_id = get_local_state_id_stack()

        ui.columns(2, im_str!("tab-columns"), true);

        let mut closed_tab_index = None;

        if !self.rendered {
            self.rendered = true;
            ui.set_column_width(-1, 120.0);
        }

        if tabs.is_empty() {
            id_token.pop(ui);
            return TabResult {
                selected_tab_index: None,
                closed_tab_index,
            };
        }

        let selected_tab_index = self
            .selected_tab_index
            .get_or_insert(open_tab_index.unwrap_or_default());
        let selected_tab_id = self
            .selected_tab_id
            .get_or_insert_with(|| tabs[*selected_tab_index].id.clone());

        if let Some(open_tab_index) = open_tab_index {
            *selected_tab_index = open_tab_index;
            *selected_tab_id = tabs[open_tab_index].id.clone();
        }

        // TODO: Change selected tab if windowed

        // Handle deletion/insertion
        if *selected_tab_index >= tabs.len() {
            *selected_tab_index = tabs.len() - 1;
        }
        if tabs[*selected_tab_index].id != *selected_tab_id {
            if let Some((i, _)) = tabs
                .iter()
                .enumerate()
                .find(|&(_, tab)| tab.id == *selected_tab_id)
            {
                *selected_tab_index = i;
            } else {
                *selected_tab_id = tabs[*selected_tab_index].id.clone();
            }
        }

        let windowed_tabs = &self.windowed_tabs;
        ig::ChildWindow::new("tabs").build(ui, || {
            for (i, tab) in tabs.iter().enumerate() {
                if windowed_tabs.contains(&tab.id) {
                    continue;
                }

                let selected = ig::Selectable::new(&im_str!("{}##tab-{}", tab.label, tab.id))
                    .selected(*selected_tab_id == tab.id)
                    .build(ui);
                if selected {
                    *selected_tab_index = i;
                    *selected_tab_id = tab.id.clone();
                }

                if tab.closable
                    && ui.is_item_hovered()
                    && ui.is_mouse_clicked(ig::MouseButton::Middle)
                {
                    closed_tab_index = Some(i);
                }

                if allow_windowing || tab.closable {
                    // TODO: Context menu
                    //    if ig.begin_popup_context_item(f'##ctx-{tab.id}'):
                    //      if allow_windowing and ig.selectable('Pop out')[0]:
                    //        windowed_tabs.add(tab.id)
                    //      if tab.closable and ig.selectable('Close')[0]:
                    //        closed_tab = i
                    //      ig.end_popup_context_item()
                }
            }
        });

        ui.next_column();

        ig::ChildWindow::new("content")
            .flags(ig::WindowFlags::HORIZONTAL_SCROLLBAR)
            .build(ui, || {
                let tab = &mut tabs[*selected_tab_index];
                if !windowed_tabs.contains(&tab.id) {
                    (tab.render)(&tab.id);
                }
            });

        ui.columns(1, im_str!("tab-columns-end"), false);

        for tab_id in &self.windowed_tabs.clone() {
            if let Some(tab) = tabs.iter_mut().find(|tab| tab.id == *tab_id) {
                let style_token =
                    ui.push_style_color(ig::StyleColor::WindowBg, [0.06, 0.06, 0.06, 0.94]);

                let mut opened = true;
                ig::Window::new(&im_str!("{}##window-{}", tab.label, tab.id))
                    .size(ui.window_size(), ig::Condition::Once)
                    .position(ui.window_pos(), ig::Condition::Once)
                    .opened(&mut opened)
                    .flags(ig::WindowFlags::HORIZONTAL_SCROLLBAR)
                    .build(ui, || {
                        (tab.render)(&tab.id);
                    });

                style_token.pop(ui);

                if !opened {
                    self.windowed_tabs.remove(tab_id);
                }
            } else {
                self.windowed_tabs.remove(tab_id);
            }
        }

        id_token.pop(ui);
        TabResult {
            selected_tab_index: if open_tab_index == Some(*selected_tab_index) {
                None
            } else {
                Some(*selected_tab_index)
            },
            closed_tab_index,
        }
    }
}
