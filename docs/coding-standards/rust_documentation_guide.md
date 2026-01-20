# Rust Library Documentation Guide

**Complete guide to creating, building, and publishing professional documentation for Rust libraries**

Version: 1.0 (January 2025)

---

## Table of Contents

1. [Introduction](#introduction)
2. [API Documentation with rustdoc](#api-documentation-with-rustdoc)
3. [Automatic Publishing to docs.rs](#automatic-publishing-to-docsrs)
4. [Project Documentation with mdBook](#project-documentation-with-mdbook)
5. [Continuous Integration & Deployment](#continuous-integration--deployment)
6. [Documentation Structure & Organization](#documentation-structure--organization)
7. [Best Practices & Checklist](#best-practices--checklist)

---

## Introduction

Rust documentation ecosystem provides powerful tools for creating both API documentation and narrative guides:

- **rustdoc**: Built-in tool for API documentation from source code comments
- **docs.rs**: Automatic hosting for crate documentation
- **mdBook**: Tool for creating book-style documentation (guides, tutorials, etc.)
- **GitHub Actions**: CI/CD for automated documentation builds and deployment

### Documentation Goals

✅ **DO**: Provide comprehensive, searchable API documentation  
✅ **DO**: Include working examples that users can copy  
✅ **DO**: Maintain guides and tutorials for common use cases  
✅ **DO**: Automate documentation builds and deployment  
✅ **DO**: Keep documentation synchronized with code changes  

---

## API Documentation with rustdoc

### Basic Usage

rustdoc is Rust's built-in documentation generator. It extracts documentation from special comments in your source code.

#### Documentation Comments

```rust
/// This is a doc comment for the item that follows
/// 
/// # Examples
/// 
/// ```
/// use my_crate::foo;
/// 
/// let result = foo(42);
/// assert_eq!(result, 84);
/// ```
pub fn foo(x: i32) -> i32 {
    x * 2
}

//! This is an inner doc comment for the containing item (e.g., module or crate)
//! 
//! Use this at the top of lib.rs to document your entire crate.
```

#### Building Documentation

```bash
# Build documentation for your crate
cargo doc

# Build and open in browser
cargo doc --open

# Include private items
cargo doc --document-private-items

# Build documentation for all dependencies
cargo doc --no-deps  # Opposite: only your crate

# Build for specific features
cargo doc --features "feature1,feature2"

# Build with all features enabled
cargo doc --all-features
```

#### Output Location

Documentation is generated in `target/doc/[crate_name]/index.html`

### Documentation Sections

Every well-documented item should include:

#### 1. Summary Line

```rust
/// Computes the factorial of a number using iterative algorithm.
```

First line should be a concise summary (appears in search results).

#### 2. Extended Description

```rust
/// Computes the factorial of a number using iterative algorithm.
/// 
/// This implementation uses an iterative approach rather than recursion
/// to avoid stack overflow for large values. Returns `None` if the
/// result would overflow `u64`.
```

#### 3. Examples Section

```rust
/// # Examples
/// 
/// Basic usage:
/// 
/// ```
/// use my_crate::factorial;
/// 
/// assert_eq!(factorial(5), Some(120));
/// assert_eq!(factorial(0), Some(1));
/// ```
/// 
/// Handling overflow:
/// 
/// ```
/// # use my_crate::factorial;
/// assert_eq!(factorial(100), None);  // Would overflow
/// ```
```

**Note**: Lines starting with `#` in code blocks are compiled but hidden in docs.

#### 4. Errors Section

```rust
/// # Errors
/// 
/// Returns `Err` if:
/// 
/// - The file does not exist
/// - The process lacks permissions to read the file
/// - The file contains invalid UTF-8
```

#### 5. Panics Section

```rust
/// # Panics
/// 
/// Panics if the index is out of bounds.
```

#### 6. Safety Section (for unsafe functions)

```rust
/// # Safety
/// 
/// The caller must ensure that:
/// 
/// - The pointer is valid and properly aligned
/// - The data behind the pointer is initialized
/// - No other references exist to the same memory
```

### Doc Attributes

```rust
// Link to other items
/// See also: [`Config`], [`Client::connect`]

// Hide implementation details
#[doc(hidden)]
pub struct InternalHelper;

// Inline documentation from another item
#[doc(inline)]
pub use some_module::ImportantType;

// Configure documentation
#![doc(html_logo_url = "https://example.com/logo.png")]
#![doc(html_favicon_url = "https://example.com/favicon.ico")]
#![doc(html_root_url = "https://docs.rs/my-crate/0.1.0")]
```

### Intra-doc Links

Use backticks for inline code and square brackets for links:

```rust
/// This uses the [`Config`] struct and calls [`Client::connect`].
/// 
/// You can also link to modules: [`crate::utils`]
/// External crates: [`std::fs::File`]
/// Methods: [`Vec::push`]
/// Associated items: [`Option::Some`]
```

### Documentation Tests

Code blocks in documentation are automatically tested:

```rust
/// ```
/// let x = 5;
/// assert_eq!(x, 5);
/// ```
```

Control test behavior:

```rust
/// ```no_run
/// // This code is compiled but not run
/// std::process::exit(0);
/// ```

/// ```ignore
/// // This code is not compiled or run
/// This is not valid Rust
/// ```

/// ```should_panic
/// // This test should panic
/// panic!("expected");
/// ```

/// ```compile_fail
/// // This should fail to compile
/// let x: i32 = "not a number";
/// ```
```

### Cargo.toml Metadata for Documentation

```toml
[package]
name = "my-crate"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
edition = "2021"
description = "A brief description of what your crate does"
documentation = "https://docs.rs/my-crate"  # Optional: only if not using docs.rs
homepage = "https://my-crate.com"  # Optional: project website
repository = "https://github.com/username/my-crate"
readme = "README.md"
keywords = ["keyword1", "keyword2"]  # Max 5 keywords
categories = ["category1"]  # See https://crates.io/categories
license = "MIT OR Apache-2.0"

# Enable all features when building docs
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

### Conditional Documentation

```rust
// Code only included in documentation
#[cfg(doc)]
use std::collections::HashMap;

// Code excluded from documentation
#[cfg(not(doc))]
use fast_hashmap::HashMap;

// Detect docs.rs specifically
#[cfg(docsrs)]
mod extra_docs;
```

---

## Automatic Publishing to docs.rs

### How docs.rs Works

1. **Automatic Builds**: When you publish to crates.io, docs.rs automatically builds your documentation
2. **Queue System**: Crates are built in order; check the [build queue](https://docs.rs/releases/queue)
3. **Multiple Targets**: Documentation is built for multiple platforms by default
4. **Free Hosting**: All documentation is hosted permanently at `https://docs.rs/[crate-name]`

### Configuring docs.rs Builds

Add configuration to `Cargo.toml`:

```toml
[package.metadata.docs.rs]
# Features to enable during docs.rs build (default: [])
features = ["feature1", "feature2"]

# Build with all features (default: false)
all-features = true

# Build without default features (default: false)
no-default-features = true

# Default target platform (default: "x86_64-unknown-linux-gnu")
default-target = "x86_64-unknown-linux-gnu"

# Additional targets to build documentation for
# Default targets are automatically built:
# - x86_64-unknown-linux-gnu
# - aarch64-apple-darwin  
# - x86_64-pc-windows-msvc
# - aarch64-unknown-linux-gnu
# - i686-pc-windows-msvc
targets = ["wasm32-unknown-unknown"]

# Or add targets without removing defaults
additional-targets = ["i686-apple-darwin"]

# Additional rustc flags
rustc-args = ["--cfg", "my_cfg"]

# Additional rustdoc flags
rustdoc-args = ["--cfg", "docsrs", "--html-in-header", "docs-header.html"]

# Additional cargo arguments (options only, not subcommands)
cargo-args = ["-Z", "build-std"]
```

### Detecting docs.rs Environment

In Rust code:

```rust
#[cfg(docsrs)]
pub mod docs_only_module {
    //! This module only appears in docs.rs documentation
}

// Example: show different docs on docs.rs
#[cfg_attr(docsrs, doc = "Documentation for docs.rs")]
#[cfg_attr(not(docsrs), doc = "Documentation for local builds")]
pub fn my_function() {}
```

In `build.rs`:

```rust
fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        println!("cargo:rustc-cfg=docsrs");
        // Skip expensive build steps when building docs
    }
}
```

### README on docs.rs

docs.rs displays your README automatically:

```toml
[package]
# Use README.md (default if file exists)
readme = "README.md"

# Use a different file
readme = "docs/crate-readme.md"

# Explicitly use README.md
readme = true

# Disable README display
readme = false
```

### Resource Limits

docs.rs builds have the following limits:

| Resource | Limit |
|----------|-------|
| RAM | 6.44 GB |
| Build timeout | 15 minutes |
| Build log size | 102.4 kB |
| Network access | Blocked |
| Max targets | 10 |

To request limit increases, [open an issue](https://github.com/rust-lang/docs.rs/issues).

### Testing docs.rs Builds Locally

Install cargo-docs-rs:

```bash
cargo install cargo-docs-rs
```

Test your documentation build:

```bash
# Test with docs.rs environment
cargo docs-rs

# This simulates the docs.rs build environment
```

### Troubleshooting Build Failures

Common issues:

1. **Missing Dependencies**: Add to [crates-build-env](https://github.com/rust-lang/crates-build-env)
2. **Write to Read-only Directories**: Use `OUT_DIR` environment variable in `build.rs`
3. **Timeout**: Reduce build targets or request an extension
4. **Feature Incompatibilities**: Test locally with `cargo docs-rs`

Rebuild from crates.io:

1. Go to your crate page on crates.io
2. Click the version number
3. Click "Rebuild documentation"

---

## Project Documentation with mdBook

mdBook creates beautiful, searchable documentation books (like the official Rust Book).

### Installation

```bash
# Install mdBook
cargo install mdbook

# Verify installation
mdbook --version
```

### Creating a New Book

```bash
# Create new book structure
mdbook init my-book

# This creates:
# my-book/
# ├── book.toml       # Configuration
# └── src/
#     ├── SUMMARY.md  # Table of contents
#     └── chapter_1.md
```

### Project Structure

```
my-project/
├── Cargo.toml
├── src/            # Rust source code
├── docs/           # mdBook documentation
│   ├── book.toml
│   └── src/
│       ├── SUMMARY.md
│       ├── introduction.md
│       ├── guide/
│       │   ├── getting-started.md
│       │   └── advanced.md
│       └── api/
│           └── reference.md
└── README.md
```

### Configuration (book.toml)

```toml
[book]
title = "My Crate Guide"
authors = ["Your Name"]
description = "Complete guide to using my-crate"
src = "src"
language = "en"

[build]
build-dir = "book"  # Output directory

[output.html]
# Site URL (important for 404 page)
site-url = "/my-crate/"

# Git repository link
git-repository-url = "https://github.com/username/my-crate"
git-repository-icon = "fa-github"

# Edit button
edit-url-template = "https://github.com/username/my-crate/edit/main/docs/{path}"

# Additional CSS
additional-css = ["theme/custom.css"]

# Additional JS
additional-js = ["theme/custom.js"]

# Theme colors
[output.html.theme]
default = "light"

# Search settings
[output.html.search]
enable = true
limit-results = 30
teaser-word-count = 30

# Code highlighting
[output.html.highlight]
theme = "base16-ocean-dark"

# Playground
[output.html.playground]
runnable = true
```

### Table of Contents (SUMMARY.md)

```markdown
# Summary

[Introduction](introduction.md)

# User Guide

- [Getting Started](guide/getting-started.md)
  - [Installation](guide/installation.md)
  - [Quick Start](guide/quick-start.md)
- [Core Concepts](guide/concepts.md)
- [Advanced Usage](guide/advanced.md)

# API Reference

- [Client](api/client.md)
- [Configuration](api/config.md)

# Contributing

- [Development Setup](contributing/setup.md)
- [Testing](contributing/testing.md)

[Changelog](changelog.md)
```

### Building and Serving

```bash
# Build the book
mdbook build

# Serve with hot-reload
mdbook serve

# Open in browser
mdbook serve --open

# Serve on custom port
mdbook serve --port 8080

# Build and run tests
mdbook test
```

### Markdown Features

```markdown
# Standard Markdown

## Code Blocks with Highlighting

\`\`\`rust
fn main() {
    println!("Hello, world!");
}
\`\`\`

## Hidden Lines

\`\`\`rust
# fn main() {
#     // This line is hidden in the rendered output
    println!("Visible line");
# }
\`\`\`

## Runnable Examples

\`\`\`rust,editable
fn main() {
    println!("Click play to run!");
}
\`\`\`

## File Includes

{{#include ../Cargo.toml}}

## Code Includes

{{#rustdoc_include ../src/lib.rs:example}}

## Links

See [Chapter 2](chapter-2.md) for more details.

## Admonitions (with CSS)

> **Note**: This is important information.

> **Warning**: Be careful with this!
```

### Testing Code Examples

mdBook can test Rust code blocks:

```bash
# Test all code examples
mdbook test

# Test specific chapter
mdbook test path/to/chapter.md
```

### Custom 404 Page

Create `src/404.md`:

```markdown
# Page Not Found

The page you're looking for doesn't exist.

[Return to Home](index.md)
```

### Preprocessors

mdBook supports preprocessors for custom transformations:

```toml
[preprocessor.links]
# Link checker preprocessor

[preprocessor.index]
# Generate index
```

### Popular Preprocessors

```bash
# Link checking
cargo install mdbook-linkcheck

# Mermaid diagrams
cargo install mdbook-mermaid

# Table of contents
cargo install mdbook-toc

# PDF generation
cargo install mdbook-pdf
```

Configure in `book.toml`:

```toml
[preprocessor.linkcheck]
# Check for broken links

[preprocessor.mermaid]
# Enable Mermaid diagrams
```

---

## Continuous Integration & Deployment

### GitHub Actions Workflows

#### 1. API Documentation Deployment

Create `.github/workflows/docs.yml`:

```yaml
name: Documentation

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Build documentation
        run: cargo doc --no-deps --all-features
        env:
          RUSTDOCFLAGS: "--cfg docsrs"
      
      - name: Add redirect
        run: echo '<meta http-equiv="refresh" content="0; url=my_crate">' > target/doc/index.html
      
      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: target/doc

  deploy:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

#### 2. mdBook Deployment

Create `.github/workflows/book.yml`:

```yaml
name: Deploy Book

on:
  push:
    branches: [main]
    paths:
      - 'docs/**'
      - '.github/workflows/book.yml'

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install mdBook
        run: |
          mkdir bin
          curl -sSL https://github.com/rust-lang/mdBook/releases/download/v0.4.40/mdbook-v0.4.40-x86_64-unknown-linux-gnu.tar.gz | tar -xz --directory=bin
          echo "$(pwd)/bin" >> $GITHUB_PATH
      
      - name: Build book
        run: mdbook build docs
      
      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: docs/book

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

#### 3. Combined Documentation

Create `.github/workflows/docs-combined.yml`:

```yaml
name: Documentation

on:
  push:
    branches: [main]

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install mdBook
        run: |
          mkdir -p ~/bin
          curl -sSL https://github.com/rust-lang/mdBook/releases/download/v0.4.40/mdbook-v0.4.40-x86_64-unknown-linux-gnu.tar.gz | tar -xz --directory=~/bin
          echo "$HOME/bin" >> $GITHUB_PATH
      
      - name: Build API documentation
        run: cargo doc --no-deps --all-features
        env:
          RUSTDOCFLAGS: "--cfg docsrs"
      
      - name: Build guide
        run: mdbook build docs
      
      - name: Combine documentation
        run: |
          mkdir -p public
          cp -r target/doc/* public/api/
          cp -r docs/book/* public/
          echo '<a href="api/">API Documentation</a>' > public/api.html
      
      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: public

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

#### 4. Documentation Testing

Create `.github/workflows/docs-test.yml`:

```yaml
name: Documentation Tests

on:
  pull_request:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Check documentation
        run: |
          cargo doc --no-deps --all-features
          cargo test --doc
        env:
          RUSTDOCFLAGS: "-D warnings"
      
      - name: Install mdBook
        run: |
          mkdir -p ~/bin
          curl -sSL https://github.com/rust-lang/mdBook/releases/download/v0.4.40/mdbook-v0.4.40-x86_64-unknown-linux-gnu.tar.gz | tar -xz --directory=~/bin
          echo "$HOME/bin" >> $GITHUB_PATH
      
      - name: Test book
        run: mdbook test docs
      
      - name: Check links (optional)
        run: |
          cargo install mdbook-linkcheck
          mdbook build docs
```

### GitLab CI/CD

Create `.gitlab-ci.yml`:

```yaml
stages:
  - build
  - deploy

build-docs:
  stage: build
  image: rust:latest
  script:
    - cargo doc --no-deps --all-features
    - mkdir -p public
    - cp -r target/doc/* public/
  artifacts:
    paths:
      - public
  only:
    - main

pages:
  stage: deploy
  script:
    - echo "Deploying documentation"
  artifacts:
    paths:
      - public
  only:
    - main
```

### Local Pre-publish Checklist Script

Create `scripts/check-docs.sh`:

```bash
#!/bin/bash
set -e

echo "==> Checking documentation..."

# Check that docs build without warnings
echo "Building documentation..."
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Run doc tests
echo "Running doc tests..."
cargo test --doc

# Check for broken intra-doc links
echo "Checking for broken links..."
cargo rustdoc -- -D rustdoc::broken-intra-doc-links

# If mdBook is used
if [ -d "docs" ]; then
    echo "Building book..."
    mdbook build docs
    
    echo "Testing book..."
    mdbook test docs
fi

echo "==> Documentation checks passed!"
```

Make it executable:

```bash
chmod +x scripts/check-docs.sh
```

---

## Documentation Structure & Organization

### Recommended Project Layout

```
my-crate/
├── Cargo.toml
├── README.md                 # Crate overview (shown on crates.io and docs.rs)
├── CHANGELOG.md              # Version history
├── src/
│   ├── lib.rs               # Crate-level documentation
│   ├── client.rs            # Module documentation
│   └── config.rs
├── examples/
│   ├── basic.rs             # Runnable examples
│   └── advanced.rs
├── docs/                    # mdBook guide (optional but recommended)
│   ├── book.toml
│   └── src/
│       ├── SUMMARY.md
│       ├── introduction.md
│       ├── guide/
│       │   ├── installation.md
│       │   ├── quick-start.md
│       │   └── configuration.md
│       ├── tutorials/
│       │   └── building-app.md
│       ├── api/
│       │   └── overview.md
│       └── contributing.md
├── .github/
│   └── workflows/
│       └── docs.yml         # CI/CD for documentation
└── scripts/
    └── check-docs.sh        # Local documentation validation
```

### Crate-level Documentation (lib.rs)

```rust
//! # My Crate
//!
//! `my_crate` provides efficient data structures for X.
//!
//! ## Quick Start
//!
//! ```
//! use my_crate::Client;
//!
//! let client = Client::new();
//! client.connect()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Features
//!
//! - **Fast**: Optimized for performance
//! - **Safe**: Built with Rust's safety guarantees
//! - **Flexible**: Configurable for various use cases
//!
//! ## Optional Features
//!
//! - `tls`: Enable TLS support (requires OpenSSL)
//! - `async`: Enable async/await support
//!
//! ## Examples
//!
//! See the [examples directory] for more use cases.
//!
//! [examples directory]: https://github.com/username/my-crate/tree/main/examples

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Rest of lib.rs
```

### Module-level Documentation

```rust
//! Client implementation module.
//!
//! This module provides the main [`Client`] type and related functionality.
//!
//! # Examples
//!
//! ```
//! use my_crate::client::Client;
//!
//! let client = Client::builder()
//!     .timeout(30)
//!     .build();
//! ```

use std::time::Duration;

/// A client for connecting to the service.
///
/// The client maintains a connection pool and handles automatic reconnection.
pub struct Client {
    // ...
}
```

### README.md Structure

```markdown
# My Crate

[![Crates.io](https://img.shields.io/crates/v/my-crate.svg)](https://crates.io/crates/my-crate)
[![Documentation](https://docs.rs/my-crate/badge.svg)](https://docs.rs/my-crate)
[![Build Status](https://github.com/username/my-crate/workflows/CI/badge.svg)](https://github.com/username/my-crate/actions)

Brief description of what your crate does.

## Features

- Feature 1
- Feature 2
- Feature 3

## Quick Start

\`\`\`rust
use my_crate::Client;

let client = Client::new();
\`\`\`

## Documentation

- [API Documentation](https://docs.rs/my-crate)
- [User Guide](https://username.github.io/my-crate/)
- [Examples](examples/)

## Installation

Add to your `Cargo.toml`:

\`\`\`toml
[dependencies]
my-crate = "0.1"
\`\`\`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
```

### Examples Directory

Provide runnable examples in `examples/`:

```rust
// examples/basic.rs
//! Basic usage example
//!
//! Run with: cargo run --example basic

use my_crate::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    
    // Demonstrate basic usage
    client.connect()?;
    
    Ok(())
}
```

Users can run with: `cargo run --example basic`

---

## Best Practices & Checklist

### Documentation Quality Checklist

#### Code Documentation

- [ ] Every public item has documentation
- [ ] All documentation includes examples
- [ ] Examples are tested (via `cargo test --doc`)
- [ ] Error conditions are documented
- [ ] Panic conditions are documented
- [ ] Unsafe code has safety documentation
- [ ] Intra-doc links work correctly
- [ ] Documentation builds without warnings
- [ ] Feature-gated items are properly documented

#### Project Documentation

- [ ] README.md is comprehensive and up-to-date
- [ ] CHANGELOG.md is maintained
- [ ] Installation instructions are clear
- [ ] Quick start guide exists
- [ ] Common use cases are covered
- [ ] Troubleshooting section exists
- [ ] Contributing guidelines are clear

#### Build Configuration

- [ ] Cargo.toml metadata is complete
- [ ] docs.rs configuration is optimized
- [ ] All features are documented
- [ ] Documentation builds on docs.rs
- [ ] CI/CD tests documentation
- [ ] GitHub Pages is set up (if applicable)

### Writing Style Guide

✅ **DO**:
- Use clear, concise language
- Write in present tense
- Use active voice
- Include practical examples
- Link to related items
- Document edge cases
- Provide context for decisions

❌ **DON'T**:
- Assume prior knowledge
- Use jargon without explanation
- Write overly terse descriptions
- Copy-paste without adaptation
- Leave TODOs in published docs
- Include implementation details in public API docs

### Example Quality Standards

✅ **Good Example**:
```rust
/// Parses a configuration file.
///
/// # Examples
///
/// ```
/// use my_crate::Config;
///
/// let config = Config::from_file("config.toml")?;
/// assert_eq!(config.port, 8080);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns an error if the file doesn't exist or contains invalid TOML.
```

❌ **Poor Example**:
```rust
/// Parses config
pub fn parse_config(path: &str) -> Result<Config, Error> {
    // No example, no error documentation
}
```

### Documentation Maintenance

1. **Update with Code Changes**: Documentation should be updated in the same commit
2. **Version Appropriately**: Document breaking changes in CHANGELOG.md
3. **Review Regularly**: Audit documentation quarterly for accuracy
4. **Test Examples**: Run `cargo test --doc` before every release
5. **Check Links**: Use link checkers in CI
6. **Monitor docs.rs**: Check that builds succeed after publishing

### Performance Tips

- Use `cargo doc --no-deps` to skip dependency docs
- Enable only needed features during docs builds
- Cache rustdoc builds in CI
- Use `--document-private-items` sparingly (only for internal docs)

### Accessibility

- Use descriptive link text
- Provide alt text for images
- Structure content with proper headings
- Ensure code examples have sufficient context
- Test with screen readers if possible

---

## Additional Resources

### Official Documentation

- [The rustdoc Book](https://doc.rust-lang.org/rustdoc/)
- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [The mdBook Guide](https://rust-lang.github.io/mdBook/)
- [docs.rs About Page](https://docs.rs/about)

### Tools

- [cargo-docs-rs](https://github.com/dtolnay/cargo-docs-rs) - Test docs.rs builds locally
- [mdbook-linkcheck](https://github.com/Michael-F-Bryan/mdbook-linkcheck) - Check for broken links
- [cargo-rdme](https://github.com/orium/cargo-rdme) - Generate README from rustdoc
- [cargo-watch](https://github.com/watchexec/cargo-watch) - Auto-rebuild docs on change

### Examples of Great Documentation

- [serde](https://serde.rs/) - Comprehensive guide + API docs
- [tokio](https://tokio.rs/) - Tutorial-focused approach
- [clap](https://docs.rs/clap/) - Extensive examples
- [diesel](http://diesel.rs/) - Getting started guide

---

## Version History

- **1.0** (January 2025): Initial comprehensive guide

## Contributing

This guide is part of the Rust client library standards. Contributions and corrections are welcome.

---

**Next Steps**: Use this guide alongside the [Rust Client Library Standards](rust_client_library_standards.md) document to create production-quality Rust libraries with excellent documentation.
