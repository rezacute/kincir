use crate::Message;
use async_trait::async_trait;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

#[async_trait]
pub trait Logger: Send + Sync {
    async fn info(&self, msg: &str);
    async fn error(&self, msg: &str);
}

pub struct StdLogger {
    show_info: bool,
    show_error: bool,
}

impl StdLogger {
    pub fn new(show_info: bool, show_error: bool) -> Self {
        Self {
            show_info,
            show_error,
        }
    }
}

#[async_trait]
impl Logger for StdLogger {
    async fn info(&self, msg: &str) {
        if self.show_info {
            println!("[INFO] {}", msg);
        }
    }

    async fn error(&self, msg: &str) {
        if self.show_error {
            eprintln!("[ERROR] {}", msg);
        }
    }
}

pub type HandlerFunc = Arc<dyn Fn(Message) -> Pin<Box<dyn Future<Output = Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>;

pub struct Router {
    logger: Arc<dyn Logger>,
    consume_topic: String,
    publish_topic: String,
    subscriber: Arc<dyn crate::Subscriber<Error = Box<dyn std::error::Error + Send + Sync>>>,
    publisher: Arc<dyn crate::Publisher<Error = Box<dyn std::error::Error + Send + Sync>>>,
    handler: HandlerFunc,
}

impl Router {
    pub fn new(
        logger: Arc<dyn Logger>,
        consume_topic: String,
        publish_topic: String,
        subscriber: Arc<dyn crate::Subscriber<Error = Box<dyn std::error::Error + Send + Sync>>>,
        publisher: Arc<dyn crate::Publisher<Error = Box<dyn std::error::Error + Send + Sync>>>,
        handler: HandlerFunc,
    ) -> Self {
        Self {
            logger,
            consume_topic,
            publish_topic,
            subscriber,
            publisher,
            handler,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Starting router...").await;
        self.subscriber.subscribe(&self.consume_topic).await?;

        loop {
            match self.subscriber.receive().await {
                Ok(msg) => {
                    self.logger
                        .info(&format!("Received message: {}", msg.uuid))
                        .await;

                    match (self.handler)(msg).await {
                        Ok(processed_msgs) => {
                            if !processed_msgs.is_empty() {
                                self.publisher
                                    .publish(&self.publish_topic, processed_msgs)
                                    .await?;
                            }
                        }
                        Err(e) => {
                            self.logger
                                .error(&format!("Error processing message: {}", e))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    self.logger
                        .error(&format!("Error receiving message: {}", e))
                        .await;
                }
            }
        }
    }
}