use kincir::Message;
use kincir::protobuf::{MessageCodec, ProtobufCodec};

fn main() {
    // Create a message with payload and metadata
    let original_message = Message::new(b"Hello, Protocol Buffers!".to_vec())
        .with_metadata("content-type", "text/plain")
        .with_metadata("priority", "high")
        .with_metadata("timestamp", &chrono::Utc::now().to_rfc3339());
    
    println!("Original message:");
    println!("  UUID: {}", original_message.uuid);
    println!("  Payload: {} bytes", original_message.payload.len());
    println!("  Metadata entries: {}", original_message.metadata.len());
    
    // Create a protobuf codec
    let codec = ProtobufCodec::new();
    
    // Encode the message
    let encoded = codec.encode(&original_message).unwrap();
    println!("\nMessage encoded to {} bytes using Protocol Buffers", encoded.len());
    
    // Decode the message
    let decoded_message = codec.decode(&encoded).unwrap();
    
    // Verify the decoded message matches the original
    println!("\nDecoded message:");
    println!("  UUID: {}", decoded_message.uuid);
    println!("  Payload: {} bytes", decoded_message.payload.len());
    println!("  Metadata entries: {}", decoded_message.metadata.len());
    
    // Check if the messages are equivalent
    let uuid_match = original_message.uuid == decoded_message.uuid;
    let payload_match = original_message.payload == decoded_message.payload;
    let metadata_match = original_message.metadata == decoded_message.metadata;
    
    println!("\nVerification:");
    println!("  UUID match: {}", uuid_match);
    println!("  Payload match: {}", payload_match);
    println!("  Metadata match: {}", metadata_match);
    
    if uuid_match && payload_match && metadata_match {
        println!("\nSuccess! The message was correctly encoded and decoded using Protocol Buffers.");
    } else {
        println!("\nError: The decoded message doesn't match the original.");
    }
} 