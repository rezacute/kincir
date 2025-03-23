# Kincir Documentation Site

This directory contains the documentation site for Kincir, a unified message streaming library for Rust. The site is built using Jekyll and deployed using GitHub Pages.

## Local Development

To run the site locally, you'll need Ruby and Bundler installed. Then:

1. Install dependencies:
```bash
bundle install
```

2. Start the Jekyll server:
```bash
bundle exec jekyll serve
```

3. Visit `http://localhost:4000` in your browser to preview the site.

### Alternative: Using Python Server

If you're having issues with Jekyll, you can use the included Python server to preview the static files:

```bash
python3 serve.py
```

Then visit `http://localhost:8000` in your browser.

## Documentation Structure

- **Home Page**: Landing page with key features and links
- **Documentation**: Comprehensive guides and API documentation
  - Installation
  - Quick Start
  - Core Concepts
  - Backend Integrations
  - API Reference

## Contributing to Documentation

1. Fork the repository
2. Create a new branch
3. Make your changes
4. Submit a pull request

## Adding New Pages

To add a new documentation page:

1. Create a new markdown file in the appropriate directory under `/docs`
2. Add front matter with layout and title:
```yaml
---
layout: default
title: Your Page Title
---
```
3. Write your content using Markdown

## Deployment

The site is automatically deployed to GitHub Pages when changes are pushed to the main branch. The deployment is handled by GitHub Actions using the workflow defined in `.github/workflows/jekyll.yml`.

### Deployment Process

1. When changes are pushed to the main branch, the GitHub Actions workflow is triggered
2. The workflow sets up Ruby, installs dependencies, and builds the Jekyll site
3. The built site is then deployed to GitHub Pages
4. The deployed site is available at https://rezacute.github.io/kincir/ 