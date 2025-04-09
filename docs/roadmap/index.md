---
layout: default
title: Roadmap
---

<div class="roadmap-container">
  <h1>Roadmap</h1>

  <p class="intro-text">The Kincir project is actively being developed. Here's our roadmap for future releases:</p>

  <h2>Current Version (0.2.x)</h2>

  <div class="roadmap-item completed">
    <div class="roadmap-icon">✓</div>
    <div class="roadmap-content">
      <h3>Basic message routing</h3>
    </div>
  </div>

  <div class="roadmap-item completed">
    <div class="roadmap-icon">✓</div>
    <div class="roadmap-content">
      <h3>Middleware support</h3>
    </div>
  </div>

  <div class="roadmap-item completed">
    <div class="roadmap-icon">✓</div>
    <div class="roadmap-content">
      <h3>Async handler support</h3>
    </div>
  </div>

  <div class="roadmap-item completed">
    <div class="roadmap-icon">✓</div>
    <div class="roadmap-content">
      <h3>Feature flag system</h3>
    </div>
  </div>

  <div class="roadmap-item completed">
    <div class="roadmap-icon">✓</div>
    <div class="roadmap-content">
      <h3>Context passing</h3>
    </div>
  </div>

  <h2>Next Release (0.3.x)</h2>

  <div class="roadmap-item in-progress">
    <div class="roadmap-icon">⟳</div>
    <div class="roadmap-content">
      <h3>Enhanced error handling</h3>
      <ul>
        <li>Custom error types</li>
        <li>Error middleware</li>
        <li>Result-based handler return values</li>
      </ul>
    </div>
  </div>

  <div class="roadmap-item in-progress">
    <div class="roadmap-icon">⟳</div>
    <div class="roadmap-content">
      <h3>Improved middleware API</h3>
      <ul>
        <li>Before/after middleware separation</li>
        <li>Conditional middleware execution</li>
      </ul>
    </div>
  </div>

  <div class="roadmap-item in-progress">
    <div class="roadmap-icon">⟳</div>
    <div class="roadmap-content">
      <h3>Message transformation pipelines</h3>
    </div>
  </div>

  <h2>Future Releases (0.4.x and beyond)</h2>

  <div class="roadmap-item planned">
    <div class="roadmap-icon">○</div>
    <div class="roadmap-content">
      <h3>Message scheduling and delay</h3>
    </div>
  </div>

  <div class="roadmap-item planned">
    <div class="roadmap-icon">○</div>
    <div class="roadmap-content">
      <h3>Batched message handling</h3>
    </div>
  </div>

  <div class="roadmap-item planned">
    <div class="roadmap-icon">○</div>
    <div class="roadmap-content">
      <h3>Message persistence</h3>
    </div>
  </div>

  <div class="roadmap-item planned">
    <div class="roadmap-icon">○</div>
    <div class="roadmap-content">
      <h3>Distributed routing</h3>
    </div>
  </div>

  <div class="roadmap-item planned">
    <div class="roadmap-icon">○</div>
    <div class="roadmap-content">
      <h3>Performance optimizations</h3>
      <ul>
        <li>Message queue improvements</li>
        <li>Async performance enhancements</li>
      </ul>
    </div>
  </div>

  <div class="roadmap-item planned">
    <div class="roadmap-icon">○</div>
    <div class="roadmap-content">
      <h3>Additional integrations</h3>
      <ul>
        <li>Tracing/OpenTelemetry support</li>
        <li>More ecosystem integration (actix, axum, etc.)</li>
      </ul>
    </div>
  </div>

  <h2>Long-term Vision</h2>

  <p>Our long-term goal for Kincir is to provide a robust, high-performance message routing system for Rust applications that is:</p>

  <div class="vision-items">
    <div class="vision-item">
      <h3>Flexible</h3>
      <p>Works with any type of message and handler</p>
    </div>
    
    <div class="vision-item">
      <h3>Extensible</h3>
      <p>Easy to extend with custom middleware and plugins</p>
    </div>
    
    <div class="vision-item">
      <h3>Performant</h3>
      <p>Fast and efficient for high-throughput applications</p>
    </div>
    
    <div class="vision-item">
      <h3>Scalable</h3>
      <p>Able to scale from small applications to large distributed systems</p>
    </div>
    
    <div class="vision-item">
      <h3>Well-documented</h3>
      <p>Comprehensive documentation and examples</p>
    </div>
  </div>

  <div class="roadmap-footer">
    <h2>Contributing</h2>

    <p>We welcome contributions to help us achieve this roadmap faster! Check out our <a href="{{ site.github.repository_url }}/blob/main/CONTRIBUTING.md">contribution guidelines</a> if you'd like to get involved.</p>

    <h2>Feature Requests</h2>

    <p>Have a feature you'd like to see in Kincir? Please submit a feature request on our <a href="{{ site.github.repository_url }}/issues/new?labels=enhancement&template=feature_request.md">GitHub issues page</a>.</p>
  </div>
</div> 