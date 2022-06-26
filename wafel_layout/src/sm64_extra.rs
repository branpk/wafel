use std::convert::TryFrom;

use indexmap::IndexMap;
use serde_json::{Map, Value as JsonValue};
use wafel_data_type::{DataType, DataTypeRef, Field, IntValue, Namespace, TypeName};

use crate::{
    Constant, ConstantSource, DataLayout,
    SM64LayoutError::{self, *},
};

impl DataLayout {
    /// Load hardcoded SM64 data into the data layout.
    ///
    /// This includes:
    /// - rawData fields in the Object struct
    /// - Integer constants defined by macros
    pub fn add_sm64_extras(&mut self) -> Result<(), SM64LayoutError> {
        let json: JsonValue = serde_json::from_slice(include_bytes!("../sm64_macro_defns.json"))
            .expect("failed to deserialize sm64 macro defns");
        let fields = as_object(&json);
        let constants_json = field(&fields, "constants");
        let object_fields_json = field(&fields, "object_fields");
        load_object_fields(self, &object_fields_json)?;
        load_constants(self, &constants_json);
        Ok(())
    }
}

/// Load virtual object fields from the given json string.
fn load_object_fields(layout: &mut DataLayout, json: &JsonValue) -> Result<(), SM64LayoutError> {
    let object_struct_ref = layout.data_type_mut(&TypeName {
        namespace: Namespace::Struct,
        name: "Object".to_owned(),
    })?;

    let object_struct = DataTypeRef::get_mut(object_struct_ref).ok_or(ObjectStructInUse)?;
    if let DataType::Struct { fields } = object_struct {
        let extra_fields = read_object_fields(&json, fields)?;
        fields.extend(extra_fields);
        Ok(())
    } else {
        Err(ObjectStructNotStruct)
    }
}

/// Load constants from the given json string.
fn load_constants(layout: &mut DataLayout, json: &JsonValue) {
    for (name, info) in as_object(&json) {
        let info = as_object(info);
        let value_field = field(info, "value");

        let value = if let Some(value) = value_field.as_i64() {
            IntValue::from(value)
        } else {
            continue;
        };

        layout.constants.insert(
            name.clone(),
            Constant {
                value,
                source: ConstantSource::Macro,
            },
        );
    }
}

fn read_object_fields(
    json: &JsonValue,
    object_struct_fields: &IndexMap<String, Field>,
) -> Result<IndexMap<String, Field>, SM64LayoutError> {
    let raw_data_field = match object_struct_fields.get("rawData") {
        Some(field) => field,
        None => return Err(MissingRawData),
    };
    let raw_data_arrays = if let DataType::Union { fields } = raw_data_field.data_type.as_ref() {
        fields
    } else {
        return Err(RawDataNotUnion);
    };

    let mut object_fields: IndexMap<String, Field> = IndexMap::new();
    for (name, defn) in as_object(json) {
        let defn_fields = as_object(defn);

        let array_name = as_str(field(defn_fields, "array"));
        let mut data_type = match raw_data_arrays.get(array_name) {
            Some(array) => DataTypeRef::clone(&array.data_type),
            None => return Err(MissingRawDataArray(array_name.to_string())),
        };

        let indices: Vec<usize> = as_array(field(defn_fields, "indices"))
            .iter()
            .map(|v| as_usize(v))
            .collect();

        let mut offset = raw_data_field.offset;

        for &index in &indices {
            if let DataType::Array {
                base,
                length,
                stride,
            } = DataTypeRef::clone(&data_type).as_ref()
            {
                if let Some(length) = *length {
                    if index >= length {
                        return Err(InvalidIndex(name.clone()));
                    }
                }
                data_type = DataTypeRef::clone(base);
                offset += index * *stride;
            } else {
                return Err(InvalidIndex(name.clone()));
            }
        }

        object_fields.insert(name.clone(), Field { offset, data_type });
    }

    Ok(object_fields)
}

fn as_object(json: &JsonValue) -> &Map<String, JsonValue> {
    json.as_object()
        .unwrap_or_else(|| panic!("expect object, found: {}", json))
}

fn as_array(json: &JsonValue) -> &Vec<JsonValue> {
    json.as_array()
        .unwrap_or_else(|| panic!("expected array, found: {}", json))
}

fn as_str(json: &JsonValue) -> &str {
    json.as_str()
        .unwrap_or_else(|| panic!("expect string, found: {}", json))
}

fn as_usize(json: &JsonValue) -> usize {
    let value_u64 = json
        .as_u64()
        .unwrap_or_else(|| panic!("expect u64, found: {}", json));
    usize::try_from(value_u64).unwrap_or_else(|_| panic!("expect usize, found: {}", json))
}

fn field<'a>(object: &'a Map<String, JsonValue>, name: &str) -> &'a JsonValue {
    object
        .get(name)
        .unwrap_or_else(|| panic!("missing field {}: {:?}", name, object))
}
