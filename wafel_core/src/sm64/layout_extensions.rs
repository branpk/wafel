use super::LayoutExtensionErrorCause::*;
use crate::{
    error::Error,
    memory::{
        data_type::{DataType, DataTypeRef, Field, FloatType, IntType, Namespace, TypeName},
        Constant, ConstantSource, DataLayout, IntValue,
    },
};
use serde_json::{Map, Value as JsonValue};
use std::{collections::HashMap, convert::TryFrom};

/// Load virtual object fields from the given json string.
pub fn load_object_fields(layout: &mut DataLayout, json_source: &[u8]) -> Result<(), Error> {
    let object_struct_ref = layout.get_type_mut(&TypeName {
        namespace: Namespace::Struct,
        name: "Object".to_owned(),
    })?;

    let object_struct = DataTypeRef::get_mut(object_struct_ref).ok_or(ObjectStructInUse)?;
    if let DataType::Struct { fields } = object_struct {
        let raw_data_offset = match fields.get("rawData") {
            Some(field) => field.offset,
            None => {
                return Err(MissingRawData {
                    object_struct: object_struct_ref.clone(),
                }
                .into())
            }
        };

        let json: JsonValue = serde_json::from_slice(json_source).map_err(SerdeError)?;

        let extra_fields = read_object_fields(&json, raw_data_offset)?;
        fields.extend(extra_fields);
    } else {
        return Err(ObjectStructNotStruct {
            object_struct: object_struct_ref.clone(),
        }
        .into());
    }

    Ok(())
}

/// Load constants from the given json string.
pub fn load_constants(layout: &mut DataLayout, json_source: &[u8]) -> Result<(), Error> {
    let json: JsonValue = serde_json::from_slice(json_source).map_err(SerdeError)?;

    for (name, info) in as_object(&json)? {
        let info = as_object(info)?;
        let value_field = field(info, "value")?;
        let source_field = field(info, "source")?;

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

    Ok(())
}

fn read_object_fields(
    json: &JsonValue,
    raw_data_offset: usize,
) -> Result<HashMap<String, Field>, Error> {
    as_object(json)?
        .iter()
        .map(|(name, defn)| -> Result<_, Error> {
            let defn_fields = as_object(defn)?;
            let offset = raw_data_offset + as_usize(field(defn_fields, "offset")?)?;
            let data_type = read_type(field(defn_fields, "type")?)?;
            Ok((name.clone(), Field { offset, data_type }))
        })
        .collect()
}

fn read_type(json: &JsonValue) -> Result<DataTypeRef, Error> {
    let fields = as_object(json)?;
    let data_type = match as_str(field(fields, "kind")?)? {
        "primitive" => match as_str(field(fields, "name")?)? {
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
            let base_type_json = field(fields, "base")?;
            let base_type = read_type(base_type_json)?;
            let base_size = as_usize(field(as_object(base_type_json)?, "size")?)?;
            DataType::Pointer {
                base: base_type,
                stride: Some(base_size),
            }
        }
        "symbol" => {
            let namespace = match as_str(field(fields, "namespace")?)? {
                "struct" => Namespace::Struct,
                "union" => Namespace::Union,
                "typedef" => Namespace::Typedef,
                namespace => unimplemented!("namespace {}", namespace),
            };
            let name = as_str(field(fields, "name")?)?.to_owned();
            DataType::Name(TypeName { namespace, name })
        }
        kind => unimplemented!("kind {}", kind),
    };
    Ok(DataTypeRef::new(data_type))
}

fn as_object(json: &JsonValue) -> Result<&Map<String, JsonValue>, Error> {
    json.as_object().ok_or_else(|| {
        WrongType {
            expected: "object".to_owned(),
            value: json.to_string(),
        }
        .into()
    })
}

fn as_str(json: &JsonValue) -> Result<&str, Error> {
    json.as_str().ok_or_else(|| {
        WrongType {
            expected: "string".to_owned(),
            value: json.to_string(),
        }
        .into()
    })
}

fn as_usize(json: &JsonValue) -> Result<usize, Error> {
    let value_u64 = json.as_u64().ok_or_else(|| WrongType {
        expected: "u64".to_owned(),
        value: json.to_string(),
    })?;
    let value_usize = usize::try_from(value_u64).map_err(|_| WrongType {
        expected: "usize".to_owned(),
        value: json.to_string(),
    })?;
    Ok(value_usize)
}

fn field<'a>(object: &'a Map<String, JsonValue>, name: &str) -> Result<&'a JsonValue, Error> {
    object.get(name).ok_or_else(|| {
        MissingField {
            object: format!("{:?}", object),
            field: name.to_owned(),
        }
        .into()
    })
}
