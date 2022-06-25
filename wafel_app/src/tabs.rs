use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use imgui::{self as ig, im_str};
use once_cell::sync::OnceCell;

pub(crate) struct TabInfo {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) closable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TabResult {
    pub(crate) selected_tab_index: Option<usize>,
    pub(crate) closed_tab_index: Option<usize>,
}

#[derive(Debug, Default)]
struct TabsState {
    rendered: bool,
    selected_tab_index: Option<usize>,
    selected_tab_id: Option<String>,
    windowed_tabs: HashSet<String>,
}

fn tabs_state() -> &'static Mutex<HashMap<ig::sys::ImGuiID, TabsState>> {
    static INSTANCE: OnceCell<Mutex<HashMap<ig::sys::ImGuiID, TabsState>>> = OnceCell::new();
    INSTANCE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn render_tabs(
    ui: &ig::Ui<'_>,
    id: &str,
    tabs: &[TabInfo],
    open_tab_index: Option<usize>,
    allow_windowing: bool,
    mut render: impl FnMut(usize),
) -> TabResult {
    let id_token = ui.push_id(id);

    let state_id = unsafe { ig::sys::igGetID_Str(im_str!("state").as_ptr()) };
    let mut global_tabs_state = tabs_state().lock().unwrap();
    let state = global_tabs_state.entry(state_id).or_default();

    ui.columns(2, im_str!("tab-columns"), true);

    let mut closed_tab_index = None;

    if !state.rendered {
        state.rendered = true;
        ui.set_column_width(-1, 120.0);
    }

    if tabs.is_empty() {
        id_token.pop();
        return TabResult {
            selected_tab_index: None,
            closed_tab_index,
        };
    }

    let selected_tab_index = state
        .selected_tab_index
        .get_or_insert(open_tab_index.unwrap_or_default());
    let selected_tab_id = state
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

    let windowed_tabs = &mut state.windowed_tabs;
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

            if tab.closable && ui.is_item_hovered() && ui.is_mouse_clicked(ig::MouseButton::Middle)
            {
                closed_tab_index = Some(i);
            }

            if allow_windowing || tab.closable {
                unsafe {
                    if ig::sys::igBeginPopupContextItem(im_str!("##ctx-{}", tab.id).as_ptr(), 1) {
                        if allow_windowing && ig::Selectable::new(im_str!("Pop out")).build(ui) {
                            windowed_tabs.insert(tab.id.clone());
                        } else if tab.closable && ig::Selectable::new(im_str!("Close")).build(ui) {
                            closed_tab_index = Some(i);
                        }
                        ig::sys::igEndPopup();
                    }
                }
            }
        }
    });

    ui.next_column();

    ig::ChildWindow::new("content")
        .flags(ig::WindowFlags::HORIZONTAL_SCROLLBAR)
        .build(ui, || {
            let tab = &tabs[*selected_tab_index];
            if !windowed_tabs.contains(&tab.id) {
                render(*selected_tab_index);
            }
        });

    ui.columns(1, im_str!("tab-columns-end"), false);

    for tab_id in &state.windowed_tabs.clone() {
        if let Some(tab_index) = tabs.iter().position(|tab| tab.id == *tab_id) {
            let tab = &tabs[tab_index];

            let style_token =
                ui.push_style_color(ig::StyleColor::WindowBg, [0.06, 0.06, 0.06, 0.94]);

            let mut opened = true;
            ig::Window::new(&im_str!("{}##window-{}", tab.label, tab.id))
                .size(ui.window_size(), ig::Condition::Once)
                .position(ui.window_pos(), ig::Condition::Once)
                .opened(&mut opened)
                .flags(ig::WindowFlags::HORIZONTAL_SCROLLBAR)
                .build(ui, || {
                    render(tab_index);
                });

            style_token.pop();

            if !opened {
                state.windowed_tabs.remove(tab_id);
            }
        } else {
            state.windowed_tabs.remove(tab_id);
        }
    }

    id_token.pop();
    TabResult {
        selected_tab_index: if open_tab_index == Some(*selected_tab_index) {
            None
        } else {
            Some(*selected_tab_index)
        },
        closed_tab_index,
    }
}
