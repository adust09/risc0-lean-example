use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::trace_types::ValueId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Value {
    Scalar(u64),
    Object {
        tag: u16,
        fields: Vec<Value>,
        scalars: Vec<u8>,
    },
    Array(Vec<Value>),
    ByteArray(Vec<u8>),
    Closure {
        fn_id: String,
        arity: u32,
        captured: Vec<Value>,
    },
    Nat(Vec<u8>), // big-endian bytes for arbitrary-precision nat
    Str(String),
    Irrelevant,
}

impl Value {
    pub fn tag(&self) -> u16 {
        match self {
            Value::Object { tag, .. } => *tag,
            Value::Scalar(v) => *v as u16,
            _ => 0,
        }
    }

    pub fn field(&self, idx: usize) -> &Value {
        match self {
            Value::Object { fields, .. } => {
                fields.get(idx).unwrap_or(&Value::Irrelevant)
            }
            // Scalars, Irrelevant, etc. - return Irrelevant for field access
            // This happens when a boxed value is projected
            _ => &Value::Irrelevant,
        }
    }

    pub fn set_field(&mut self, idx: usize, val: Value) {
        match self {
            Value::Object { fields, .. } => {
                if idx < fields.len() {
                    fields[idx] = val;
                }
            }
            _ => {}
        }
    }

    pub fn get_scalar_bytes(&self, _n: u32, offset: u32, size: usize) -> Vec<u8> {
        match self {
            Value::Object { scalars, .. } => {
                let start = offset as usize;
                let end = (start + size).min(scalars.len());
                if start < scalars.len() {
                    let mut result = scalars[start..end].to_vec();
                    result.resize(size, 0);
                    result
                } else {
                    vec![0u8; size]
                }
            }
            Value::Scalar(v) => {
                let bytes = v.to_le_bytes();
                let start = offset as usize;
                let mut result = vec![0u8; size];
                for i in 0..size {
                    if start + i < 8 {
                        result[i] = bytes[start + i];
                    }
                }
                result
            }
            _ => vec![0u8; size],
        }
    }

    pub fn set_scalar_bytes(&mut self, _n: u32, offset: u32, data: &[u8]) {
        match self {
            Value::Object { scalars, .. } => {
                let start = offset as usize;
                let end = start + data.len();
                if scalars.len() < end {
                    scalars.resize(end, 0);
                }
                scalars[start..end].copy_from_slice(data);
            }
            _ => {}
        }
    }

    pub fn as_u64(&self) -> u64 {
        match self {
            Value::Scalar(v) => *v,
            Value::Irrelevant => 0,
            Value::Object { tag, .. } => *tag as u64,
            _ => panic!("as_u64 on non-scalar: {:?}", self),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Value::Scalar(v) => *v != 0,
            Value::Object { tag, .. } => *tag != 0,
            _ => false,
        }
    }

    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        // Simple serialization for output commitment
        match self {
            Value::ByteArray(data) => data.clone(),
            Value::Scalar(v) => v.to_le_bytes().to_vec(),
            _ => {
                let json = serde_json::to_vec(self).unwrap_or_default();
                json
            }
        }
    }

    pub fn flatten_into(
        &self,
        table: &mut Vec<FlatValue>,
        dedup: &mut HashMap<FlatValue, ValueId>,
    ) -> ValueId {
        let flat = match self {
            Value::Scalar(v) => FlatValue::Scalar(*v),
            Value::Object {
                tag,
                fields,
                scalars,
            } => {
                let field_ids: Vec<ValueId> =
                    fields.iter().map(|f| f.flatten_into(table, dedup)).collect();
                FlatValue::Object {
                    tag: *tag,
                    fields: field_ids,
                    scalars: scalars.clone(),
                }
            }
            Value::Array(elems) => {
                let elem_ids: Vec<ValueId> =
                    elems.iter().map(|e| e.flatten_into(table, dedup)).collect();
                FlatValue::Array(elem_ids)
            }
            Value::ByteArray(data) => FlatValue::ByteArray(data.clone()),
            Value::Closure {
                fn_id,
                arity,
                captured,
            } => {
                let cap_ids: Vec<ValueId> = captured
                    .iter()
                    .map(|c| c.flatten_into(table, dedup))
                    .collect();
                FlatValue::Closure {
                    fn_id: fn_id.clone(),
                    arity: *arity,
                    captured: cap_ids,
                }
            }
            Value::Nat(bytes) => FlatValue::Nat(bytes.clone()),
            Value::Str(s) => FlatValue::Str(s.clone()),
            Value::Irrelevant => FlatValue::Irrelevant,
        };

        if let Some(&existing_id) = dedup.get(&flat) {
            return existing_id;
        }
        let id = table.len() as ValueId;
        table.push(flat.clone());
        dedup.insert(flat, id);
        id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlatValue {
    Scalar(u64),
    Object {
        tag: u16,
        fields: Vec<ValueId>,
        scalars: Vec<u8>,
    },
    Array(Vec<ValueId>),
    ByteArray(Vec<u8>),
    Closure {
        fn_id: String,
        arity: u32,
        captured: Vec<ValueId>,
    },
    Nat(Vec<u8>),
    Str(String),
    Irrelevant,
}

impl FlatValue {
    pub fn tag(&self) -> u16 {
        match self {
            FlatValue::Object { tag, .. } => *tag,
            FlatValue::Scalar(v) => *v as u16,
            _ => 0,
        }
    }

    pub fn as_u64(&self) -> u64 {
        match self {
            FlatValue::Scalar(v) => *v,
            FlatValue::Irrelevant => 0,
            FlatValue::Object { tag, .. } => *tag as u64,
            _ => panic!("as_u64 on non-scalar FlatValue: {:?}", self),
        }
    }

    pub fn field_id(&self, idx: usize) -> Option<ValueId> {
        match self {
            FlatValue::Object { fields, .. } => fields.get(idx).copied(),
            _ => None,
        }
    }

    pub fn with_field_set(&self, idx: usize, val_id: ValueId) -> FlatValue {
        match self {
            FlatValue::Object {
                tag,
                fields,
                scalars,
            } => {
                let mut new_fields = fields.clone();
                if idx < new_fields.len() {
                    new_fields[idx] = val_id;
                }
                FlatValue::Object {
                    tag: *tag,
                    fields: new_fields,
                    scalars: scalars.clone(),
                }
            }
            other => other.clone(),
        }
    }

    pub fn serialize_to_bytes(&self, _table: &[FlatValue]) -> Vec<u8> {
        match self {
            FlatValue::ByteArray(data) => data.clone(),
            FlatValue::Scalar(v) => v.to_le_bytes().to_vec(),
            _ => {
                // For complex types, serialize as JSON via serde
                serde_json::to_vec(self).unwrap_or_default()
            }
        }
    }
}

pub fn reconstruct(table: &[FlatValue], id: ValueId) -> Value {
    match &table[id as usize] {
        FlatValue::Scalar(v) => Value::Scalar(*v),
        FlatValue::Object {
            tag,
            fields,
            scalars,
        } => {
            let rec_fields: Vec<Value> = fields.iter().map(|fid| reconstruct(table, *fid)).collect();
            Value::Object {
                tag: *tag,
                fields: rec_fields,
                scalars: scalars.clone(),
            }
        }
        FlatValue::Array(elems) => {
            let rec_elems: Vec<Value> = elems.iter().map(|eid| reconstruct(table, *eid)).collect();
            Value::Array(rec_elems)
        }
        FlatValue::ByteArray(data) => Value::ByteArray(data.clone()),
        FlatValue::Closure {
            fn_id,
            arity,
            captured,
        } => {
            let rec_captured: Vec<Value> =
                captured.iter().map(|cid| reconstruct(table, *cid)).collect();
            Value::Closure {
                fn_id: fn_id.clone(),
                arity: *arity,
                captured: rec_captured,
            }
        }
        FlatValue::Nat(bytes) => Value::Nat(bytes.clone()),
        FlatValue::Str(s) => Value::Str(s.clone()),
        FlatValue::Irrelevant => Value::Irrelevant,
    }
}
