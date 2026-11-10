use std::collections::HashMap;

/// Shared NBT representation for SNBT text queries and binary save files
#[derive(Debug, Clone)]
pub enum Nbt {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    List(Vec<Nbt>),
    Compound(HashMap<String, Nbt>),
    ByteArray(Vec<i8>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Nbt {
    pub fn get(&self, key: &str) -> Option<&Nbt> {
        match self {
            Nbt::Compound(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        Some(match self {
            Nbt::Byte(v) => *v as f64,
            Nbt::Short(v) => *v as f64,
            Nbt::Int(v) => *v as f64,
            Nbt::Long(v) => *v as f64,
            Nbt::Float(v) => *v as f64,
            Nbt::Double(v) => *v,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Nbt::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> &[Nbt] {
        match self {
            Nbt::List(items) => items,
            _ => &[],
        }
    }

    pub fn num(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(Nbt::as_f64)
    }
}

impl From<fastnbt::Value> for Nbt {
    fn from(value: fastnbt::Value) -> Self {
        use fastnbt::Value as V;
        match value {
            V::Byte(v) => Nbt::Byte(v),
            V::Short(v) => Nbt::Short(v),
            V::Int(v) => Nbt::Int(v),
            V::Long(v) => Nbt::Long(v),
            V::Float(v) => Nbt::Float(v),
            V::Double(v) => Nbt::Double(v),
            V::String(v) => Nbt::String(v),
            V::ByteArray(v) => Nbt::ByteArray(v.into_inner()),
            V::IntArray(v) => Nbt::IntArray(v.into_inner()),
            V::LongArray(v) => Nbt::LongArray(v.into_inner()),
            V::List(items) => Nbt::List(items.into_iter().map(Nbt::from).collect()),
            V::Compound(map) => {
                Nbt::Compound(map.into_iter().map(|(k, v)| (k, Nbt::from(v))).collect())
            }
        }
    }
}
