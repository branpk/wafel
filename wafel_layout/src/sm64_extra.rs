use std::{collections::HashMap, convert::TryFrom};

use serde_json::{Map, Value as JsonValue};
use wafel_data_type::{
    DataType, DataTypeRef, Field, FloatType, IntType, IntValue, Namespace, TypeName,
};

use crate::{
    Constant, ConstantSource, DataLayout,
    SM64ExtrasError::{self, *},
};

pub fn load_sm64_extras(layout: &mut DataLayout) -> Result<(), SM64ExtrasError> {
    load_object_fields(layout, include_bytes!("../sm64_extras/object_fields.json"))?;
    load_constants(layout, include_bytes!("../sm64_extras/constants.json"));
    Ok(())
}

/// Load virtual object fields from the given json string.
fn load_object_fields(layout: &mut DataLayout, json_source: &[u8]) -> Result<(), SM64ExtrasError> {
    let object_struct_ref = layout.data_type_mut(&TypeName {
        namespace: Namespace::Struct,
        name: "Object".to_owned(),
    })?;

    let object_struct = DataTypeRef::get_mut(object_struct_ref).ok_or(ObjectStructInUse)?;
    if let DataType::Struct { fields } = object_struct {
        let raw_data_offset = match fields.get("rawData") {
            Some(field) => field.offset,
            None => return Err(MissingRawData),
        };

        let json: JsonValue =
            serde_json::from_slice(json_source).expect("failed to deserialize sm64 object fields");

        let extra_fields = read_object_fields(&json, raw_data_offset);
        fields.extend(extra_fields);
        Ok(())
    } else {
        Err(ObjectStructNotStruct)
    }
}

/// Load constants from the given json string.
fn load_constants(layout: &mut DataLayout, json_source: &[u8]) {
    let json: JsonValue =
        serde_json::from_slice(json_source).expect("failed to deserialize sm64 constants");

    for (name, info) in as_object(&json) {
        let info = as_object(info);
        let value_field = field(info, "value");
        let source_field = field(info, "source");

        let value = if let Some(value) = value_field.as_i64() {
            IntValue::from(value)
        } else {
            continue;
        };

        let source = match source_field.as_str() {
            Some("macro") => ConstantSource::Macro,
            _ => unimplemented!("{:?}", source_field),
        };

        layout
            .constants
            .insert(name.clone(), Constant { value, source });
    }
}

fn read_object_fields(json: &JsonValue, raw_data_offset: usize) -> HashMap<String, Field> {
    as_object(json)
        .iter()
        .map(|(name, defn)| {
            let defn_fields = as_object(defn);
            let offset = raw_data_offset + as_usize(field(defn_fields, "offset"));
            let data_type = read_type(field(defn_fields, "type"));
            (name.clone(), Field { offset, data_type })
        })
        .collect()
}

fn read_type(json: &JsonValue) -> DataTypeRef {
    let fields = as_object(json);
    let data_type = match as_str(field(fields, "kind")) {
        "primitive" => match as_str(field(fields, "name")) {
            "void" => DataType::Void,
            "u8" => DataType::Int(IntType::U8),
            "s8" => DataType::Int(IntType::S8),
            "u16" => DataType::Int(IntType::U16),
            "s16" => DataType::Int(IntType::S16),
            "u32" => DataType::Int(IntType::U32),
            "s32" => DataType::Int(IntType::S32),
            "u64" => DataType::Int(IntType::U64),
            "s64" => DataType::Int(IntType::S64),
            "f32" => DataType::Float(FloatType::F32),
            "f64" => DataType::Float(FloatType::F64),
            name => unimplemented!("primitive {}", name),
        },
        "pointer" => {
            let base_type_json = field(fields, "base");
            let base_type = read_type(base_type_json);
            let base_size = as_usize(field(as_object(base_type_json), "size"));
            DataType::Pointer {
                base: base_type,
                stride: Some(base_size),
            }
        }
        "symbol" => {
            let namespace = match as_str(field(fields, "namespace")) {
                "struct" => Namespace::Struct,
                "union" => Namespace::Union,
                "typedef" => Namespace::Typedef,
                namespace => unimplemented!("namespace {}", namespace),
            };
            let name = as_str(field(fields, "name")).to_owned();
            DataType::Name(TypeName { namespace, name })
        }
        kind => unimplemented!("kind {}", kind),
    };
    DataTypeRef::new(data_type)
}

fn as_object(json: &JsonValue) -> &Map<String, JsonValue> {
    json.as_object()
        .unwrap_or_else(|| panic!("expect object, found: {}", json))
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
