//! gRPC frame parsing and manipulation

use super::{GrpcDirection, GrpcFrame, ProtoField, ProtoMessage, ProtoValue};
use crate::Error;

/// gRPC frame header (5 bytes)
///
/// Format:
/// - 1 byte: compression flag
/// - 4 bytes: message length (big-endian)
#[derive(Debug, Clone)]
pub struct GrpcFrameHeader {
    pub compressed: bool,
    pub length: u32,
}

impl GrpcFrameHeader {
    /// Size of the frame header in bytes
    pub const SIZE: usize = 5;

    /// Parse a frame header from bytes
    pub fn parse(data: &[u8]) -> crate::Result<Option<Self>> {
        if data.len() < Self::SIZE {
            return Ok(None);
        }

        let compressed = data[0] != 0;
        let length = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);

        Ok(Some(Self { compressed, length }))
    }

    /// Encode the header to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = vec![if self.compressed { 1 } else { 0 }];
        buf.extend_from_slice(&self.length.to_be_bytes());
        buf
    }
}

/// Parse a complete gRPC frame from bytes.
///
/// If the frame is compressed (gzip), the payload is automatically
/// decompressed before being stored in the `GrpcFrame`.
pub fn parse_frame(
    data: &[u8],
    stream_id: &str,
    connection_id: &str,
    direction: GrpcDirection,
) -> crate::Result<Option<(GrpcFrame, usize)>> {
    let header = match GrpcFrameHeader::parse(data)? {
        Some(h) => h,
        None => return Ok(None),
    };

    let total_len = GrpcFrameHeader::SIZE + header.length as usize;
    if data.len() < total_len {
        return Ok(None);
    }

    let frame_data = &data[GrpcFrameHeader::SIZE..total_len];

    // Decompress gzip-compressed frames
    let payload = if header.compressed {
        decompress_gzip(frame_data)?
    } else {
        frame_data.to_vec()
    };

    let frame = GrpcFrame::new(
        stream_id,
        connection_id,
        direction,
        payload,
        header.compressed,
    );

    Ok(Some((frame, total_len)))
}

/// Decompress a gzip-compressed gRPC message payload.
fn decompress_gzip(data: &[u8]) -> crate::Result<Vec<u8>> {
    use std::io::Read;
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut out = Vec::with_capacity(data.len() * 2);
    decoder
        .read_to_end(&mut out)
        .map_err(|e| crate::Error::Proxy(format!("gRPC gzip decompression failed: {}", e)))?;
    Ok(out)
}

/// Build a gRPC frame from data
pub fn build_frame(data: &[u8], compressed: bool) -> Vec<u8> {
    let header = GrpcFrameHeader {
        compressed,
        length: data.len() as u32,
    };
    let mut buf = header.encode();
    buf.extend_from_slice(data);
    buf
}

/// Protobuf wire types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireType {
    Varint = 0,
    Fixed64 = 1,
    LengthDelimited = 2,
    StartGroup = 3,
    EndGroup = 4,
    Fixed32 = 5,
}

impl TryFrom<u8> for WireType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Varint),
            1 => Ok(Self::Fixed64),
            2 => Ok(Self::LengthDelimited),
            3 => Ok(Self::StartGroup),
            4 => Ok(Self::EndGroup),
            5 => Ok(Self::Fixed32),
            _ => Err(Error::Proxy(format!("Invalid wire type: {}", value))),
        }
    }
}

/// Decode a protobuf message (basic parsing without schema)
pub fn decode_protobuf(data: &[u8]) -> crate::Result<ProtoMessage> {
    let mut fields = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Read field tag (varint)
        let (tag, tag_len) = read_varint(&data[pos..])?;
        pos += tag_len;

        let field_number = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u8;

        let wire_type = WireType::try_from(wire_type)?;

        let (value, value_len) = read_field_value(&data[pos..], wire_type)?;
        pos += value_len;

        fields.push(ProtoField {
            number: field_number,
            wire_type: wire_type as u8,
            name: None,
            value,
        });
    }

    Ok(ProtoMessage {
        message_type: None,
        fields,
        json: None,
    })
}

/// Read a varint from bytes
fn read_varint(data: &[u8]) -> crate::Result<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= data.len() {
            return Err(Error::Proxy(
                "Unexpected end of data while reading varint".into(),
            ));
        }

        let byte = data[pos];
        pos += 1;

        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
        if shift >= 64 {
            return Err(Error::Proxy("Varint too long".into()));
        }
    }

    Ok((result, pos))
}

/// Read a field value based on wire type
fn read_field_value(data: &[u8], wire_type: WireType) -> crate::Result<(ProtoValue, usize)> {
    match wire_type {
        WireType::Varint => {
            let (value, len) = read_varint(data)?;
            Ok((ProtoValue::Varint(value), len))
        }
        WireType::Fixed64 => {
            if data.len() < 8 {
                return Err(Error::Proxy("Not enough data for fixed64".into()));
            }
            let value = u64::from_le_bytes(data[..8].try_into().unwrap());
            Ok((ProtoValue::Fixed64(value), 8))
        }
        WireType::Fixed32 => {
            if data.len() < 4 {
                return Err(Error::Proxy("Not enough data for fixed32".into()));
            }
            let value = u32::from_le_bytes(data[..4].try_into().unwrap());
            Ok((ProtoValue::Fixed32(value), 4))
        }
        WireType::LengthDelimited => {
            let (len, len_size) = read_varint(data)?;
            let len = len as usize;

            if data.len() < len_size + len {
                return Err(Error::Proxy(
                    "Not enough data for length-delimited field".into(),
                ));
            }

            let field_data = &data[len_size..len_size + len];

            // Try to decode as UTF-8 string
            let value = if let Ok(s) = std::str::from_utf8(field_data) {
                ProtoValue::String(s.to_string())
            } else {
                // Try to decode as nested message
                if let Ok(nested) = decode_protobuf(field_data) {
                    ProtoValue::Nested(Box::new(nested))
                } else {
                    // Fall back to bytes
                    use base64::Engine;
                    ProtoValue::LengthDelimited(
                        base64::engine::general_purpose::STANDARD.encode(field_data),
                    )
                }
            };

            Ok((value, len_size + len))
        }
        WireType::StartGroup | WireType::EndGroup => {
            // Groups are deprecated, just skip
            Ok((ProtoValue::Group(Vec::new()), 0))
        }
    }
}

/// Try to convert protobuf message to JSON
pub fn proto_to_json(msg: &ProtoMessage) -> serde_json::Value {
    let mut obj = serde_json::Map::new();

    for field in &msg.fields {
        let key = field
            .name
            .clone()
            .unwrap_or_else(|| format!("field_{}", field.number));

        let value = match &field.value {
            ProtoValue::Varint(v) => serde_json::json!(*v),
            ProtoValue::Fixed64(v) => serde_json::json!(*v),
            ProtoValue::Fixed32(v) => serde_json::json!(*v),
            ProtoValue::String(s) => serde_json::json!(s),
            ProtoValue::Bytes(b) | ProtoValue::LengthDelimited(b) => serde_json::json!(b),
            ProtoValue::Group(_) => serde_json::json!(null),
            ProtoValue::Nested(nested) => proto_to_json(nested),
        };

        obj.insert(key, value);
    }

    serde_json::Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header() {
        let header = GrpcFrameHeader {
            compressed: false,
            length: 100,
        };
        let encoded = header.encode();
        assert_eq!(encoded.len(), 5);
        assert_eq!(encoded[0], 0);

        let parsed = GrpcFrameHeader::parse(&encoded).unwrap().unwrap();
        assert!(!parsed.compressed);
        assert_eq!(parsed.length, 100);
    }

    #[test]
    fn test_build_and_parse_frame() {
        let data = b"Hello, gRPC!";
        let frame_data = build_frame(data, false);

        let (frame, len) = parse_frame(&frame_data, "stream-1", "conn-1", GrpcDirection::Request)
            .unwrap()
            .unwrap();

        assert_eq!(len, frame_data.len());
        assert!(!frame.compressed);
        assert_eq!(frame.data_bytes().unwrap(), data.to_vec());
    }
}
