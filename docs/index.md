---
layout: default
title: Home
---

<div class="hero">
  <div class="container">
    <img src="{{ '/assets/images/kincir-logo.svg' | relative_url }}" alt="Kincir" width="100" class="hero-logo">
    <h1>Building event-driven applications <br>the easy way in Rust</h1>
    <p class="lead">
      Kincir is a unified message streaming library for Rust that provides a consistent interface for working with multiple message broker backends.
    </p>
    <div class="hero-buttons">
      <a href="{{ '/docs/' | relative_url }}" class="btn btn-primary">Get Started</a>
      <a href="{{ site.github.repository_url }}" class="btn btn-secondary">View on GitHub</a>
    </div>
  </div>
</div>

<section class="features">
  <div class="container">
    <h2 class="section-title">Key Features</h2>
    <div class="features-grid">
      <div class="feature-card">
        <h3>
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect></svg>
          Unified Interface
        </h3>
        <p>A simple, consistent API for publishing and subscribing to messages across different messaging systems.</p>
      </div>
      <div class="feature-card">
        <h3>
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg>
          Multiple Backends
        </h3>
        <p>Support for Kafka, RabbitMQ, and more message brokers with a single, consistent API.</p>
      </div>
      <div class="feature-card">
        <h3>
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 11a9 9 0 0 1 9 9"></path><path d="M4 4a16 16 0 0 1 16 16"></path><circle cx="5" cy="19" r="1"></circle></svg>
          Message Routing
        </h3>
        <p>Powerful message routing capabilities with customizable handlers for complex event processing.</p>
      </div>
      <div class="feature-card">
        <h3>
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
          Optional Features
        </h3>
        <p>Customize your build with optional feature flags for logging, Protocol Buffers support, and more.</p>
      </div>
      <div class="feature-card">
        <h3>
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"></polyline><polyline points="1 20 1 14 7 14"></polyline><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path></svg>
          Event-Driven Architecture
        </h3>
        <p>Build robust event-driven applications with reliable message passing and processing.</p>
      </div>
      <div class="feature-card">
        <h3>
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="12" width="4" height="9"></rect><rect x="10" y="8" width="4" height="13"></rect><rect x="18" y="5" width="4" height="16"></rect></svg>
          High Performance
        </h3>
        <p>Designed for performance with Rust's safety guarantees and zero-cost abstractions.</p>
      </div>
    </div>
  </div>
</section>

<section class="cta">
  <div class="container">
    <h2>Ready to start building?</h2>
    <p>Check out the documentation to learn how to integrate Kincir into your Rust applications.</p>
    <a href="{{ '/docs/' | relative_url }}" class="btn btn-primary">Get Started</a>
  </div>
</section> 