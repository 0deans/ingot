//! Low-level Binary VDF parser and serializer.

const TYPE_MAP: u8 = 0x00;
const TYPE_STRING: u8 = 0x01;
const TYPE_INT: u8 = 0x02;
const TYPE_END: u8 = 0x08;
const MAX_VDF_DEPTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VdfValue {
    Map(Vec<VdfEntry>),
    Str(String),
    Int(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VdfEntry {
    pub key: String,
    pub value: VdfValue,
}

/// Parses raw Binary VDF bytes into a lossless entry tree.
pub(super) fn parse_vdf(bytes: &[u8]) -> Result<Vec<VdfEntry>, String> {
    let mut cursor = 0usize;
    parse_map_entries(bytes, &mut cursor, 0)
}

/// Serializes Binary VDF entries back into Valve's binary VDF wire format.
pub(super) fn serialize_vdf(entries: &[VdfEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    write_map_entries(&mut out, entries);
    out.push(TYPE_END);
    out
}

fn parse_map_entries(
    bytes: &[u8],
    cursor: &mut usize,
    depth: usize,
) -> Result<Vec<VdfEntry>, String> {
    if depth > MAX_VDF_DEPTH {
        return Err("Binary VDF nesting depth exceeded limit".to_string());
    }

    let mut entries = Vec::new();
    while let Some(&tag) = bytes.get(*cursor) {
        *cursor += 1;

        if tag == TYPE_END {
            return Ok(entries);
        }

        let key = read_cstring(bytes, cursor)?;
        let value = match tag {
            TYPE_MAP => {
                let children = parse_map_entries(bytes, cursor, depth + 1)?;
                VdfValue::Map(children)
            }
            TYPE_STRING => {
                let text = read_cstring(bytes, cursor)?;
                VdfValue::Str(text)
            }
            TYPE_INT => {
                let number = read_u32_le(bytes, cursor)?;
                VdfValue::Int(number)
            }
            other => {
                return Err(format!(
                    "Unexpected binary VDF type tag 0x{other:02x} at byte offset {}",
                    cursor.saturating_sub(1)
                ));
            }
        };

        entries.push(VdfEntry { key, value });
    }

    if depth > 0 {
        return Err("Unexpected end of file inside nested binary VDF map".to_string());
    }

    Ok(entries)
}

fn read_cstring(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    let start = *cursor;
    while let Some(&byte) = bytes.get(*cursor) {
        if byte == 0 {
            let slice = bytes
                .get(start..*cursor)
                .ok_or_else(|| "Invalid string slice bounds in binary VDF".to_string())?;
            *cursor += 1;
            return Ok(String::from_utf8_lossy(slice).into_owned());
        }
        *cursor += 1;
    }
    Err("Unterminated null-delimited string in binary VDF".to_string())
}

fn read_u32_le(bytes: &[u8], cursor: &mut usize) -> Result<u32, String> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| "Integer overflow while reading binary VDF".to_string())?;
    let slice = bytes.get(*cursor..end).ok_or_else(|| {
        "Unexpected end of file while reading 32-bit integer in binary VDF".to_string()
    })?;
    let raw: [u8; 4] = slice
        .try_into()
        .map_err(|_| "Failed to read 4-byte integer in binary VDF".to_string())?;
    *cursor = end;
    Ok(u32::from_le_bytes(raw))
}

fn write_map_entries(out: &mut Vec<u8>, entries: &[VdfEntry]) {
    for entry in entries {
        match &entry.value {
            VdfValue::Map(children) => {
                out.push(TYPE_MAP);
                write_cstring(out, &entry.key);
                write_map_entries(out, children);
                out.push(TYPE_END);
            }
            VdfValue::Str(text) => {
                out.push(TYPE_STRING);
                write_cstring(out, &entry.key);
                write_cstring(out, text);
            }
            VdfValue::Int(number) => {
                out.push(TYPE_INT);
                write_cstring(out, &entry.key);
                out.extend_from_slice(&number.to_le_bytes());
            }
        }
    }
}

fn write_cstring(out: &mut Vec<u8>, text: &str) {
    for byte in text.as_bytes() {
        if *byte != 0 {
            out.push(*byte);
        }
    }
    out.push(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_nested_maps_and_strips_embedded_nul_bytes() {
        let entries = vec![VdfEntry {
            key: "root\0key".to_string(),
            value: VdfValue::Map(vec![
                VdfEntry {
                    key: "label".to_string(),
                    value: VdfValue::Str("hello\0world".to_string()),
                },
                VdfEntry {
                    key: "count".to_string(),
                    value: VdfValue::Int(0x1234_5678),
                },
            ]),
        }];

        let encoded = serialize_vdf(&entries);
        let decoded = parse_vdf(&encoded).unwrap();
        assert_eq!(
            decoded,
            vec![VdfEntry {
                key: "rootkey".to_string(),
                value: VdfValue::Map(vec![
                    VdfEntry {
                        key: "label".to_string(),
                        value: VdfValue::Str("helloworld".to_string()),
                    },
                    VdfEntry {
                        key: "count".to_string(),
                        value: VdfValue::Int(0x1234_5678),
                    },
                ]),
            }]
        );
    }

    #[test]
    fn rejects_truncated_nested_map_unterminated_string_and_excess_depth() {
        let unterminated = b"\x01key\x00value";
        assert!(parse_vdf(unterminated).is_err());

        let unclosed_map = b"\x00nested\x00\x01k\x00v\x00";
        assert!(parse_vdf(unclosed_map).is_err());

        let mut too_deep = Vec::new();
        for level in 0..=(MAX_VDF_DEPTH + 1) {
            too_deep.push(TYPE_MAP);
            too_deep.extend_from_slice(format!("m{level}\0").as_bytes());
        }
        too_deep.resize(too_deep.len() + MAX_VDF_DEPTH + 3, TYPE_END);
        assert!(parse_vdf(&too_deep).is_err());
    }
}
