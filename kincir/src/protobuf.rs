//! Protocol Buffers (protobuf) encoding and decoding for Kincir messages.
//!
//! This module provides support for encoding and decoding messages using Protocol Buffers.
//! It defines a `MessageCodec` trait for encoding/decoding operations and implementations
//! for protobuf serialization.
//!
//! # Example
//!
//! ```rust,no_run
//! use kincir::Message;
//! use kincir::protobuf::{MessageCodec, ProtobufCodec};
//!
//! // Create a protobuf codec
//! let codec = ProtobufCodec::new();
//!
//! // Encode a message
//! let msg = Message::new(b"Hello, World!".to_vec());
//! let encoded = codec.encode(&msg).unwrap();
//!
//! // Decode a message
//! let decoded = codec.decode(&encoded).unwrap();
//! ```

use crate::Message;
use prost::Message as ProstMessage;
use std::fmt::Debug;
use thiserror::Error;

/// Represents possible errors that can occur in protobuf encoding/decoding operations.
#[derive(Error, Debug)]
pub enum ProtobufError {
    /// Error when encoding a message
    #[error("Encoding error: {0}")]
    Encode(String),
    /// Error when decoding a message
    #[error("Decoding error: {0}")]
    Decode(String),
}

/// A trait for message encoding and decoding operations.
pub trait MessageCodec {
    /// The type of error that can occur during encoding/decoding operations.
    type Error;

    /// Encodes a message into a byte vector.
    fn encode(&self, message: &Message) -> Result<Vec<u8>, Self::Error>;

    /// Decodes a byte vector into a message.
    fn decode(&self, bytes: &[u8]) -> Result<Message, Self::Error>;
}

/// Protocol Buffer implementation of a message codec.
///
/// Uses Protocol Buffers for message serialization and deserialization.
pub struct ProtobufCodec;

impl Default for ProtobufCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// The Protocol Buffer representation of a Message.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ProtoMessage {
    /// Unique identifier for the message
    pub uuid: String,
    /// The actual message content as bytes
    pub payload: Vec<u8>,
    /// Additional key-value pairs associated with the message
    pub metadata: Vec<MetadataEntry>,
}

/// A single key-value entry in the message metadata.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct MetadataEntry {
    /// The key of the metadata entry
    pub key: String,
    /// The value of the metadata entry
    pub value: String,
}

impl ProstMessage for ProtoMessage {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: prost::bytes::BufMut,
    {
        // Encode uuid (field 1)
        prost::encoding::string::encode(1, &self.uuid, buf);

        // Encode payload (field 2)
        prost::encoding::bytes::encode(2, &self.payload, buf);

        // Encode metadata entries (field 3)
        for entry in &self.metadata {
            prost::encoding::message::encode(3, entry, buf);
        }
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut B,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError>
    where
        B: prost::bytes::Buf,
    {
        match tag {
            1 => prost::encoding::string::merge(wire_type, &mut self.uuid, buf, ctx),
            2 => prost::encoding::bytes::merge(wire_type, &mut self.payload, buf, ctx),
            3 => {
                let mut entry = MetadataEntry::default();
                prost::encoding::message::merge(wire_type, &mut entry, buf, ctx)?;
                self.metadata.push(entry);
                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::string::encoded_len(1, &self.uuid)
            + prost::encoding::bytes::encoded_len(2, &self.payload)
            + self
                .metadata
                .iter()
                .map(|entry| prost::encoding::message::encoded_len(3, entry))
                .sum::<usize>()
    }

    fn clear(&mut self) {
        self.uuid.clear();
        self.payload.clear();
        self.metadata.clear();
    }
}

impl ProstMessage for MetadataEntry {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: prost::bytes::BufMut,
    {
        // Encode key (field 1)
        prost::encoding::string::encode(1, &self.key, buf);

        // Encode value (field 2)
        prost::encoding::string::encode(2, &self.value, buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut B,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError>
    where
        B: prost::bytes::Buf,
    {
        match tag {
            1 => prost::encoding::string::merge(wire_type, &mut self.key, buf, ctx),
            2 => prost::encoding::string::merge(wire_type, &mut self.value, buf, ctx),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::string::encoded_len(1, &self.key)
            + prost::encoding::string::encoded_len(2, &self.value)
    }

    fn clear(&mut self) {
        self.key.clear();
        self.value.clear();
    }
}

impl ProtobufCodec {
    /// Creates a new Protocol Buffer codec.
    pub fn new() -> Self {
        Self
    }

    /// Converts a Kincir Message to a ProtoMessage.
    fn message_to_proto(&self, message: &Message) -> ProtoMessage {
        let metadata = message
            .metadata
            .iter()
            .map(|(k, v)| MetadataEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        ProtoMessage {
            uuid: message.uuid.clone(),
            payload: message.payload.clone(),
            metadata,
        }
    }

    /// Converts a ProtoMessage to a Kincir Message.
    fn proto_to_message(&self, proto: ProtoMessage) -> Message {
        let mut message = Message::new(proto.payload);
        message.uuid = proto.uuid;

        for entry in proto.metadata {
            message.metadata.insert(entry.key, entry.value);
        }

        message
    }
}

impl MessageCodec for ProtobufCodec {
    type Error = ProtobufError;

    fn encode(&self, message: &Message) -> Result<Vec<u8>, Self::Error> {
        let proto = self.message_to_proto(message);
        let mut buf = Vec::new();

        proto
            .encode(&mut buf)
            .map_err(|e| ProtobufError::Encode(e.to_string()))?;
        Ok(buf)
    }

    fn decode(&self, bytes: &[u8]) -> Result<Message, Self::Error> {
        let proto =
            ProtoMessage::decode(bytes).map_err(|e| ProtobufError::Decode(e.to_string()))?;

        Ok(self.proto_to_message(proto))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protobuf_codec_roundtrip() {
        // Create a message with metadata
        let mut original = Message::new(b"Hello, Protobuf!".to_vec());
        original = original
            .with_metadata("content-type", "text/plain")
            .with_metadata("priority", "high");

        // Create codec
        let codec = ProtobufCodec::new();

        // Encode and decode
        let encoded = codec.encode(&original).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        // Verify
        assert_eq!(original.uuid, decoded.uuid);
        assert_eq!(original.payload, decoded.payload);
        assert_eq!(original.metadata, decoded.metadata);
    }
}
