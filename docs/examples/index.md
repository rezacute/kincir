---
layout: docs
title: Examples
---

# Kincir Examples

These examples demonstrate how to use Kincir in different scenarios. Each example includes full source code and explanations.

## Basic Examples

### Simple Echo Service

This example shows how to create a basic echo service using Kincir. It creates a router that responds to text messages by sending back the same text.

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::{Router, Message, Handle};
use std::sync::Arc;

// Define a message type
#[derive(Debug)]
struct TextMessage(String);

// Define a handler
fn handle_text(msg: TextMessage) {
    println!("Received text: {}", msg.0);
}

fn main() {
    // Create a router
    let mut router = Router::new();
    
    // Register a handler
    router.on::<TextMessage, _>(handle_text);
    
    // Send a message
    router.send(TextMessage(String::from("Hello, Kincir!")));
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

### Using Middleware

Middleware allows you to intercept and process messages before and after they reach their handlers.

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::{Router, Message, Handle, middleware::Middleware, Next};

// Define a message type
#[derive(Debug)]
struct TextMessage(String);

// Create a logging middleware
fn logging_middleware<M: Message>(msg: M, next: Next<M>) {
    println!("📥 Before handling message");
    next(msg); // Pass the message to the next middleware or handler
    println!("📤 After handling message");
}

// Define a handler
fn handle_text(msg: TextMessage) {
    println!("Processing message: {}", msg.0);
}

fn main() {
    // Create a router
    let mut router = Router::new();
    
    // Register middleware (applied to all message types)
    router.use_middleware(logging_middleware);
    
    // Register a handler
    router.on::<TextMessage, _>(handle_text);
    
    // Send a message - middleware will be called first
    router.send(TextMessage(String::from("Hello with middleware!")));
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

## Advanced Examples

### Broadcast Service

This example demonstrates how to create a broadcast service that sends messages to multiple receivers.

<div class="highlight-wrapper" style="position: relative;" data-language="rust">
{% highlight rust %}
use kincir::{Router, Message, Handle};
use std::sync::Arc;

// Define a broadcast message type
#[derive(Debug, Clone)]
struct BroadcastMessage(String);

// Define handlers for different receivers
fn receiver_one(msg: BroadcastMessage) {
    println!("Receiver One got: {}", msg.0);
}

fn receiver_two(msg: BroadcastMessage) {
    println!("Receiver Two got: {}", msg.0);
}

fn main() {
    // Create a router
    let mut router = Router::new();
    
    // Register multiple handlers for the same message type
    router.on::<BroadcastMessage, _>(receiver_one);
    router.on::<BroadcastMessage, _>(receiver_two);
    
    // Send a message - both handlers will be called
    router.send(BroadcastMessage(String::from("This is a broadcast!")));
}
{% endhighlight %}
<button class="copy-button manual-copy-btn" onclick="copyCode(this)" aria-label="Copy code to clipboard">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</button>
</div>

<script>
function copyCode(button) {
  const codeBlock = button.previousElementSibling;
  const code = codeBlock.innerText;
  
  navigator.clipboard.writeText(code).then(function() {
    // Visual feedback
    button.classList.add('copied');
    button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>';
    
    // Reset after 2 seconds
    setTimeout(function() {
      button.classList.remove('copied');
      button.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
    }, 2000);
  }).catch(function(err) {
    console.error('Could not copy text: ', err);
  });
}
</script>

