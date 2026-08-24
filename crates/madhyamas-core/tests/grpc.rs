//! Integration tests for the public gRPC framing API (gated on the
//! `grpc` feature; the module is compiled out under `--no-default-features`).
#![cfg(feature = "grpc")]

use madhyamas_core::grpc::{build_frame, parse_frame, GrpcDirection, GrpcFrameHeader};

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
