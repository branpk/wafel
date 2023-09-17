use std::{fmt, sync::Arc};

use itertools::Itertools;
use once_cell::sync::OnceCell;
use wafel_api::Emu;
use wafel_data_access::MemoryLayout;
use wafel_data_type::{Address, DataType, DataTypeRef, Value};
use wafel_layout::DataLayout;
use wafel_memory::MemoryRead;

#[derive(Debug)]
pub struct DataExplorer {
    sorted_global_names: OnceCell<Vec<String>>,
    search_query: String,
}

impl DataExplorer {
    pub fn new() -> Self {
        Self {
            sorted_global_names: OnceCell::new(),
            search_query: String::new(),
        }
    }

    pub fn show(&mut self, emu: &mut Emu, ui: &mut egui::Ui) {
        let data_layout = emu.layout.data_layout();

        ui.text_edit_singleline(&mut self.search_query);
        let predicate = |name: &str| {
            self.search_query.is_empty()
                || name
                    .to_lowercase()
                    .contains(&self.search_query.to_lowercase())
        };

        ui.add_space(5.0);

        let visible_rect = ui.available_rect_before_wrap();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for name in self
                    .sorted_global_names(data_layout)
                    .iter()
                    .filter(|name| predicate(name))
                {
                    let global = data_layout.global(name).unwrap();

                    self.show_variable(
                        &emu.layout,
                        &emu.memory,
                        ui,
                        &visible_rect,
                        name,
                        &global.data_type,
                        emu.layout.symbol_address(name).ok(),
                    );
                }
            });
    }

    fn show_variable(
        &self,
        layout: &impl MemoryLayout,
        memory: &impl MemoryRead,
        ui: &mut egui::Ui,
        visible_rect: &egui::Rect,
        name: &str,
        data_type: &DataTypeRef,
        addr: Option<Address>,
    ) {
        let y = ui.next_widget_position().y;
        let is_on_screen = y + 30.0 >= visible_rect.min.y && y < visible_rect.max.y;

        let data_layout = layout.data_layout();
        let concrete_type = data_layout
            .concrete_type(data_type)
            .unwrap_or_else(|_| Arc::new(DataType::Void));
        let is_expandable = self.is_expandable(&concrete_type);

        let reader = layout.data_type_reader(&concrete_type);
        let read_value = || match (addr, &reader) {
            (Some(addr), Ok(reader)) => reader.read(memory, addr).ok(),
            _ => None,
        };

        let variable_label = if !is_on_screen {
            String::new()
        } else if self.has_simple_value(&concrete_type) {
            let value_str = match read_value() {
                Some(Value::Int(value)) => {
                    if value > 0 {
                        format!("{} = {:#X}", value, value)
                    } else if value < 0 {
                        format!("{} = -{:#X}", value, -value)
                    } else {
                        "0".to_string()
                    }
                }
                Some(value) => value.to_string(),
                None => "<error>".to_string(),
            };
            let data_type_label = self.data_type_label(data_type);
            format!("{}: {} = {}", name, data_type_label, value_str)
        } else if is_expandable && addr.is_none() {
            let data_type_label = self.data_type_label(data_type);
            format!("{}: {} = <unknown>", name, data_type_label)
        } else {
            let data_type_label = self.data_type_label(data_type);
            format!("{}: {}", name, data_type_label)
        };

        if is_expandable && addr.is_some() {
            let addr = addr.unwrap();
            egui::CollapsingHeader::new(variable_label)
                .id_source(name)
                .show(ui, |ui| match concrete_type.as_ref() {
                    DataType::Pointer { base, .. } => {
                        let deref_addr = read_value().and_then(|value| value.try_as_address().ok());
                        self.show_variable(layout, memory, ui, visible_rect, "0", base, deref_addr);
                    }
                    DataType::Array {
                        base,
                        length,
                        stride,
                    } => {
                        let length = length.unwrap_or(0);
                        for index in 0..length {
                            self.show_variable(
                                layout,
                                memory,
                                ui,
                                visible_rect,
                                &index.to_string(),
                                base,
                                Some(addr + index * stride),
                            );
                        }
                    }
                    DataType::Struct { fields } | DataType::Union { fields } => {
                        for (field_name, field) in fields {
                            self.show_variable(
                                layout,
                                memory,
                                ui,
                                visible_rect,
                                field_name,
                                &field.data_type,
                                Some(addr + field.offset),
                            );
                        }
                    }
                    _ => unreachable!(),
                });
        } else {
            ui.horizontal(|ui| {
                // Add the width of the CollapsingHeader icon
                ui.add_space(ui.style().spacing.indent);
                ui.label(variable_label);
            });
        }
    }

    fn sorted_global_names(&self, data_layout: &DataLayout) -> &[String] {
        self.sorted_global_names.get_or_init(|| {
            data_layout
                .globals
                .keys()
                .filter(|name| data_layout.global(name).unwrap().address.is_some())
                .sorted_by_key(|name| name.to_lowercase())
                .cloned()
                .collect()
        })
    }

    fn data_type_label(&self, data_type: &DataTypeRef) -> String {
        let mut s = String::new();
        self.format_data_type(&mut s, data_type).unwrap();
        s
    }

    fn format_data_type(&self, f: &mut impl fmt::Write, data_type: &DataTypeRef) -> fmt::Result {
        match data_type.as_ref() {
            DataType::Void => write!(f, "void"),
            DataType::Int(int_type) => write!(f, "{}", int_type),
            DataType::Float(float_type) => write!(f, "{}", float_type),
            DataType::Pointer { base, stride: _ } => write!(f, "ptr[{}]", base),
            DataType::Array {
                base,
                length,
                stride: _,
            } => match length {
                Some(length) => write!(f, "array[{}; {}]", base, length),
                None => write!(f, "array[{}]", base),
            },
            DataType::Struct { .. } => {
                write!(f, "struct {{ .. }}")
            }
            DataType::Union { .. } => {
                write!(f, "union {{ .. }}")
            }
            DataType::Name(name) => write!(f, "{}", name),
        }
    }

    fn is_expandable(&self, concrete_type: &DataType) -> bool {
        matches!(
            concrete_type,
            DataType::Pointer { .. }
                | DataType::Array { .. }
                | DataType::Struct { .. }
                | DataType::Union { .. }
        )
    }

    fn has_simple_value(&self, concrete_type: &DataType) -> bool {
        matches!(
            concrete_type,
            DataType::Int(_) | DataType::Float(_) | DataType::Pointer { .. }
        )
    }
}
