use kincir::Message;

#[test]
fn test_message_creation() {
    let payload = b"Hello, World!".to_vec();
    let message = Message::new(payload.clone());

    assert!(!message.uuid.is_empty(), "Message UUID should not be empty");
    assert_eq!(message.payload, payload);
}

#[test]
fn test_message_metadata() {
    let payload = b"Hello, World!".to_vec();
    let message = Message::new(payload)
        .with_metadata("content-type", "text/plain")
        .with_metadata("priority", "high");

    assert_eq!(
        message.metadata.get("content-type"),
        Some(&"text/plain".to_string())
    );
    assert_eq!(message.metadata.get("priority"), Some(&"high".to_string()));
    assert!(message.metadata.get("non-existent").is_none());
}
