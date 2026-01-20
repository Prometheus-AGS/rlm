# Rust Client Library Design Standards

> A comprehensive guide for generating idiomatic, maintainable, and user-friendly Rust client libraries.
> 
> Synthesized from Azure SDK Rust Guidelines, Rust Design Patterns, and Elegant Library APIs in Rust.

## Table of Contents

1. [Core Design Principles](#core-design-principles)
2. [API Design Standards](#api-design-standards)
3. [Type System & Safety](#type-system--safety)
4. [Error Handling](#error-handling)
5. [Async & Performance Patterns](#async--performance-patterns)
6. [Documentation & Testing](#documentation--testing)
7. [Code Generation Guidelines](#code-generation-guidelines)

---

## Core Design Principles

### Idiomatic
- Follow Rust API Guidelines and naming conventions
- Use snake_case for functions/variables, PascalCase for types
- Embrace the ecosystem with its strengths and weaknesses
- Write code that feels natural to Rust developers

### Consistent
- Client libraries should be consistent within Rust's ecosystem
- Use familiar patterns from std and popular crates
- Service-agnostic concepts (logging, HTTP, errors) should be uniform
- Maintain feature parity across API versions

### Approachable
- Provide great documentation with examples
- Use predictable defaults implementing best practices
- Make common use cases easily discoverable
- Progressive concept disclosure (simple → advanced)

### Diagnosable
- Make network calls discoverable and predictable
- Provide actionable, human-readable error messages
- Support standard debugging tools and logging
- Clear error correlation with service responses

### Dependable
- Breaking changes are extremely harmful
- Never introduce incompatibilities without strong justification
- Avoid dependencies that force compatibility issues
- Follow semantic versioning strictly

---

## API Design Standards

### Naming Conventions

#### Method Names

| Pattern | Parameters | Usage | Examples |
|---------|-----------|--------|----------|
| `new` | No self, ≥1 args | Constructor | `Client::new(endpoint, creds)` |
| `with_*` | No self, ≥1 args | Alternative constructors | `Client::with_connection_string()` |
| `from_*` | 1 arg | Conversion constructor | `Config::from_env()` |
| `as_*` | `&self` | Free conversion/view | `data.as_bytes()` |
| `to_*` | `&self` | Expensive conversion | `str.to_string()` |
| `into_*` | `self` (consumes) | Consuming conversion | `response.into_body()` |
| `is_*` | `&self` | Boolean check | `result.is_ok()` |
| `has_*` | `&self` | Boolean check | `request.has_body()` |

#### CRUD Operations

| Pattern | HTTP Method | Behavior | Example |
|---------|------------|----------|---------|
| `add_*` | POST/PUT | Add to collection, fail if exists | `add_user(user)` |
| `delete_*` | DELETE | Delete resource, don't fail if missing | `delete_user(id)` |
| `get_*` | GET | Get resource, fail if missing | `get_user(id)` |
| `list_*` | GET | Get collection (may be paged) | `list_users()` |
| `*_exists` | GET/HEAD | Check existence | `user_exists(id)` |
| `set_*` | POST/PUT | Add or update | `set_user(user)` |
| `update_*` | PATCH/PUT | Update existing, fail if missing | `update_user(id, data)` |

#### Client Naming

```rust
// ✅ DO: Service-specific client names with Client suffix
pub struct SecretClient { /* ... */ }
pub struct BlobClient { /* ... */ }

// ❌ DON'T: Generic Client names (causes import conflicts)
pub struct Client { /* ... */ }
```

### Input Parameter Flexibility

Use conversion traits to accept multiple input types:

```rust
// ✅ DO: Accept borrowed types with AsRef/Into
pub fn connect<S: AsRef<str>>(endpoint: S) -> Result<Client> {
    let endpoint = endpoint.as_ref();
    // ...
}

// ✅ DO: Use IntoIterator for collections
pub fn batch_insert<I, T>(items: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Serialize,
{
    // ...
}

// ✅ DO: Use Into<Option<T>> for optional parameters
pub fn query<S, T>(filter: S, limit: T) -> Result<Vec<Record>>
where
    S: AsRef<str>,
    T: Into<Option<usize>>,
{
    let limit = limit.into().unwrap_or(100);
    // ...
}
```

### Builder Pattern

Implement builders for complex configuration:

```rust
// ✅ DO: Provide builder pattern for complex types
pub struct ClientOptions {
    pub timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub headers: HeaderMap,
}

impl ClientOptions {
    pub fn builder() -> ClientOptionsBuilder {
        ClientOptionsBuilder::default()
    }
}

pub struct ClientOptionsBuilder {
    timeout: Option<Duration>,
    retry_policy: Option<RetryPolicy>,
    headers: HeaderMap,
}

impl ClientOptionsBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }
    
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.parse().unwrap(), value.parse().unwrap());
        self
    }
    
    pub fn build(self) -> ClientOptions {
        ClientOptions {
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
            retry_policy: self.retry_policy.unwrap_or_default(),
            headers: self.headers,
        }
    }
}
```

### Extension Traits

Extend functionality of existing types:

```rust
// ✅ DO: Use extension traits for adding methods to standard types
pub trait ResultExt<T, E> {
    fn with_context<C, F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> C,
        C: std::fmt::Display;
}

impl<T, E> ResultExt<T, E> for Result<T, E>
where
    E: std::error::Error,
{
    fn with_context<C, F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> C,
        C: std::fmt::Display,
    {
        self.map_err(|e| {
            // Add context to error
            // ...
        })
    }
}

// Usage:
client.fetch_data()
    .with_context(|| format!("Failed to fetch data for user {}", user_id))?;
```

---

## Type System & Safety

### Enumerations

```rust
// ✅ DO: Use enums instead of stringly-typed APIs
pub enum Color {
    Red,
    Green,
    Blue,
    Custom(String),
}

// ✅ DO: Implement non-exhaustive for extensible enums
#[non_exhaustive]
pub enum Status {
    Active,
    Inactive,
    Pending,
}

// ✅ DO: Derive standard traits
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Image,
    Video,
}

// ✅ DO: Use untagged variant for extensible enums
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExtensibleEnum {
    #[serde(rename = "known_value_1")]
    KnownValue1,
    #[serde(rename = "known_value_2")]
    KnownValue2,
    #[serde(untagged)]
    Unknown(String),
}
```

### Model Types

```rust
// ✅ DO: Make response models non-exhaustive
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            email: None,
            tags: Vec::new(),
            metadata: None,
        }
    }
}

// ❌ DON'T: Make request models non-exhaustive (prevents construction)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: Option<String>,
}
```

### Newtypes for Type Safety

```rust
// ✅ DO: Use newtype pattern for domain-specific types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserId(String);

impl UserId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for UserId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for UserId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
```

### Session Types for State Machines

Use the type system to encode state transitions:

```rust
// ✅ DO: Use session types for stateful APIs
pub struct Disconnected;
pub struct Connected;
pub struct Authenticated;

pub struct Connection<State> {
    endpoint: String,
    state: PhantomData<State>,
}

impl Connection<Disconnected> {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            state: PhantomData,
        }
    }
    
    pub async fn connect(self) -> Result<Connection<Connected>> {
        // Perform connection
        Ok(Connection {
            endpoint: self.endpoint,
            state: PhantomData,
        })
    }
}

impl Connection<Connected> {
    pub async fn authenticate(
        self,
        credentials: &Credentials,
    ) -> Result<Connection<Authenticated>> {
        // Perform authentication
        Ok(Connection {
            endpoint: self.endpoint,
            state: PhantomData,
        })
    }
}

impl Connection<Authenticated> {
    pub async fn send_message(&self, msg: &str) -> Result<()> {
        // Can only send when authenticated
        Ok(())
    }
}

// Usage enforces correct state transitions at compile time:
// let conn = Connection::new("ws://example.com")
//     .connect().await?
//     .authenticate(&creds).await?;
// conn.send_message("Hello").await?;
```

---

## Error Handling

### Error Types

```rust
use thiserror::Error;

// ✅ DO: Use thiserror for error definitions
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Service error: {status} - {message}")]
    Service {
        status: u16,
        message: String,
    },
    
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

// ✅ DO: Provide a Result type alias
pub type Result<T> = std::result::Result<T, ClientError>;

// ✅ DO: Implement conversion from other error types
impl From<url::ParseError> for ClientError {
    fn from(e: url::ParseError) -> Self {
        Self::InvalidConfig(e.to_string())
    }
}
```

### Error Handling Patterns

```rust
// ✅ DO: Provide context with error chains
use anyhow::{Context, Result};

pub async fn fetch_user(id: &str) -> Result<User> {
    let response = client
        .get(&format!("/users/{}", id))
        .send()
        .await
        .context("Failed to send request")?;
    
    let user = response
        .json::<User>()
        .await
        .context("Failed to parse user response")?;
    
    Ok(user)
}

// ✅ DO: Use Result for recoverable errors
pub fn parse_config(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

// ❌ DON'T: Use unwrap() or expect() in library code
// Only acceptable in examples/tests

// ✅ DO: Document panic conditions
/// Validates the input data.
///
/// # Panics
///
/// Panics if the data is internally inconsistent (should never happen).
pub fn validate(data: &Data) {
    assert!(data.is_consistent(), "Data consistency check failed");
}
```

---

## Async & Performance Patterns

### Async Client Design

```rust
// ✅ DO: Make clients async by default
pub struct ApiClient {
    client: reqwest::Client,
    base_url: Url,
}

impl ApiClient {
    pub fn new(base_url: impl AsRef<str>) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            base_url: Url::parse(base_url.as_ref())?,
        })
    }
    
    pub async fn get_resource(&self, id: &str) -> Result<Resource> {
        let url = self.base_url.join(&format!("/resources/{}", id))?;
        let response = self.client.get(url).send().await?;
        let resource = response.json().await?;
        Ok(resource)
    }
}

// ❌ DON'T: Provide synchronous alternatives
// Users can use tokio::runtime::Runtime::block_on if needed
```

### Iterator Patterns

```rust
// ✅ DO: Use iterators for lazy evaluation
pub fn parse_lines<R: BufRead>(reader: R) -> impl Iterator<Item = Result<Line>> {
    reader
        .lines()
        .map(|line| line.map_err(Into::into))
        .filter_map(|line| {
            line.ok().and_then(|l| Line::parse(&l).ok())
        })
}

// ✅ DO: Accept IntoIterator for input
pub fn batch_create<I, T>(items: I) -> Result<Vec<T>>
where
    I: IntoIterator<Item = T>,
    T: Serialize,
{
    let items: Vec<_> = items.into_iter().collect();
    // Process batch
    Ok(items)
}

// ✅ DO: Return custom iterator types
pub struct PageIterator<T> {
    client: Arc<ApiClient>,
    next_token: Option<String>,
    _phantom: PhantomData<T>,
}

impl<T> Iterator for PageIterator<T>
where
    T: DeserializeOwned,
{
    type Item = Result<Page<T>>;
    
    fn next(&mut self) -> Option<Self::Item> {
        // Implement pagination logic
        None
    }
}
```

### Pagination

```rust
// ✅ DO: Provide pager abstraction for paginated APIs
pub struct Pager<T> {
    stream: Pin<Box<dyn Stream<Item = Result<T>> + Send>>,
}

impl<T> Pager<T> {
    pub fn into_pages(self) -> PageIterator<T> {
        // Convert to page iterator
        todo!()
    }
}

impl<T> Stream for Pager<T> {
    type Item = Result<T>;
    
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}

// Usage:
let pager: Pager<User> = client.list_users().send().await?;
let users: Vec<User> = pager.try_collect().await?;

// Or paginate manually:
let mut pages = pager.into_pages();
while let Some(page) = pages.next() {
    let page = page?;
    for user in page.items {
        process(user);
    }
}
```

### Long-Running Operations

```rust
// ✅ DO: Use Poller abstraction for LROs
pub struct Poller<T> {
    stream: Pin<Box<dyn Stream<Item = Result<OperationState<T>>> + Send>>,
}

#[derive(Debug)]
pub enum OperationState<T> {
    InProgress { progress: Option<f32> },
    Succeeded(T),
    Failed(String),
    Cancelled,
}

impl<T> Poller<T> {
    pub async fn wait(self) -> Result<T> {
        // Poll until completion
        todo!()
    }
}

impl<T> Stream for Poller<T> {
    type Item = Result<OperationState<T>>;
    
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}

// ✅ DO: Prefix LRO methods with "begin_"
impl Client {
    pub async fn begin_training(&self, data: &TrainingData) -> Result<Poller<Model>> {
        // Start LRO
        todo!()
    }
}

// Usage:
let poller = client.begin_training(&data).await?;
let model = poller.wait().await?;
```

### Lazy Evaluation

```rust
// ✅ DO: Use closures for expensive optional computations
pub fn process_data<F>(data: &Data, expensive_param: F) -> Result<Output>
where
    F: FnOnce() -> ExpensiveValue,
{
    if data.needs_expensive_value() {
        let value = expensive_param();
        // Use value
        todo!()
    } else {
        // Skip expensive computation
        todo!()
    }
}

// Usage:
process_data(&data, || compute_expensive_value())?;
```

---

## Documentation & Testing

### Documentation Standards

```rust
// ✅ DO: Document all public APIs
/// Client for interacting with the Example API.
///
/// The `ExampleClient` provides methods for performing CRUD operations
/// on resources in the Example service.
///
/// # Examples
///
/// ```no_run
/// use example_client::{ExampleClient, Credentials};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ExampleClient::new(
///     "https://api.example.com",
///     Credentials::from_env()?,
/// )?;
///
/// let resource = client.get_resource("resource-id").await?;
/// println!("Resource: {:?}", resource);
/// # Ok(())
/// # }
/// ```
pub struct ExampleClient {
    // ...
}

// ✅ DO: Document errors
/// Retrieves a resource by ID.
///
/// # Arguments
///
/// * `id` - The unique identifier of the resource
///
/// # Errors
///
/// Returns an error if:
/// - The resource does not exist (404)
/// - The request is unauthorized (401)
/// - Network communication fails
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let client = example_client::ExampleClient::new("https://api.example.com", todo!())?;
/// let resource = client.get_resource("my-resource").await?;
/// # Ok(())
/// # }
/// ```
pub async fn get_resource(&self, id: &str) -> Result<Resource> {
    // ...
}

// ✅ DO: Use meaningful lifetime names
/// A borrowed reference to data valid for the `'request` lifetime.
pub struct RequestContext<'request> {
    headers: &'request HeaderMap,
    body: &'request [u8],
}

// ❌ DON'T: Use generic 'a, 'b lifetimes without context
pub struct BadContext<'a, 'b> {
    // Unclear what these lifetimes represent
}
```

### Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // ✅ DO: Write unit tests
    #[test]
    fn test_user_id_creation() {
        let id = UserId::new("user-123");
        assert_eq!(id.as_str(), "user-123");
    }
    
    // ✅ DO: Write async tests
    #[tokio::test]
    async fn test_fetch_user() {
        let client = create_test_client();
        let user = client.get_user("test-id").await.unwrap();
        assert_eq!(user.id, "test-id");
    }
    
    // ✅ DO: Test error cases
    #[tokio::test]
    async fn test_fetch_nonexistent_user() {
        let client = create_test_client();
        let result = client.get_user("nonexistent").await;
        assert!(matches!(result, Err(ClientError::Service { status: 404, .. })));
    }
    
    // ✅ DO: Use doc tests for examples
    /// ```
    /// use example_client::Config;
    ///
    /// let config = Config::default();
    /// assert_eq!(config.timeout.as_secs(), 30);
    /// ```
    pub struct Config {
        pub timeout: Duration,
    }
}
```

---

## Code Generation Guidelines

### Module Organization

```rust
// ✅ DO: Organize code into logical modules

// src/lib.rs
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod client;
pub mod error;
pub mod models;
pub mod options;

// Re-export primary types
pub use client::ExampleClient;
pub use error::{Error, Result};

// ✅ DO: Use pub(crate) for internal modules
pub(crate) mod internal {
    pub mod auth;
    pub mod retry;
}

// src/client.rs
use crate::error::Result;
use crate::models::*;
use crate::options::*;

pub struct ExampleClient {
    pub(crate) client: reqwest::Client,
    pub(crate) base_url: Url,
}

// src/models.rs
mod request;
mod response;

pub use request::*;
pub use response::*;
```

### Cargo.toml Structure

```toml
[package]
name = "example-client"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
authors = ["Your Name <you@example.com>"]
license = "MIT OR Apache-2.0"
description = "Rust client library for the Example API"
homepage = "https://github.com/user/example-client"
repository = "https://github.com/user/example-client"
documentation = "https://docs.rs/example-client"
keywords = ["api", "client", "http"]
categories = ["api-bindings", "web-programming::http-client"]

[dependencies]
# Core async
tokio = { version = "1", features = ["macros", "rt-multi-thread"], optional = true }
futures = "0.3"

# HTTP
reqwest = { version = "0.11", features = ["json"], optional = true }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
anyhow = { version = "1.0", optional = true }

# Utilities
url = "2.0"
bytes = "1.0"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
mockito = "1.0"

[features]
default = ["reqwest", "tokio"]
reqwest = ["dep:reqwest"]
```

### Feature Flags

```rust
// ✅ DO: Use feature flags for optional functionality

// Optional TLS backend
#[cfg(feature = "native-tls")]
fn create_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .use_native_tls()
        .build()
        .unwrap()
}

#[cfg(feature = "rustls")]
fn create_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .build()
        .unwrap()
}

// Optional serialization formats
#[cfg(feature = "msgpack")]
pub mod msgpack {
    pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
        rmp_serde::to_vec(value).map_err(Into::into)
    }
}
```

### Safety and Lints

```rust
// ✅ DO: Set strict lints in lib.rs
#![warn(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
)]
#![deny(unsafe_code)]

// ✅ DO: Only use unsafe when absolutely necessary
// and document why it's needed
/// # Safety
///
/// The caller must ensure that `ptr` is valid for reads of
/// `len` bytes and that the memory is properly aligned.
pub unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> &'static [u8] {
    std::slice::from_raw_parts(ptr, len)
}
```

### Workspace Configuration

```toml
# Root Cargo.toml for workspace

[workspace]
members = [
    "client",
    "models",
    "auth",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
reqwest = { version = "0.11", default-features = false }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
anyhow = "1.0"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
```

---

## Checklist for Generated Code

### Client Implementation

- [ ] Client struct has `Client` suffix and service-specific name
- [ ] Client fields use `pub(crate)` visibility
- [ ] Client implements `Debug` (or custom Debug avoiding PII)
- [ ] Client methods are immutable (`&self`)
- [ ] Async methods are preferred, no sync alternatives
- [ ] Builder pattern for complex configuration
- [ ] Proper error handling with custom error types

### Type Definitions

- [ ] Response models are `#[non_exhaustive]`
- [ ] Request models are NOT `#[non_exhaustive]`
- [ ] All types derive `Clone`, `Debug` as appropriate
- [ ] Enums use `#[non_exhaustive]` for extensibility
- [ ] Serde rename rules for camelCase/snake_case
- [ ] Optional fields use `Option<T>`
- [ ] Collection fields use `Vec<T>` (not `Option<Vec<T>>`)

### API Design

- [ ] Methods follow naming conventions (get_, list_, set_, etc.)
- [ ] Input parameters use `AsRef<str>` / `Into<T>` where appropriate
- [ ] Iterators use `IntoIterator` trait for parameters
- [ ] Return types use `Result<T, E>` consistently
- [ ] Paginated APIs return `Pager<T>`
- [ ] LROs prefixed with `begin_` and return `Poller<T>`

### Documentation

- [ ] All public APIs have doc comments
- [ ] Examples included in doc comments
- [ ] Error conditions documented
- [ ] Panics documented if any
- [ ] README included via `include_str!`
- [ ] Module-level documentation provided

### Testing

- [ ] Unit tests for core logic
- [ ] Integration tests in `tests/` directory
- [ ] Doc tests compile and run
- [ ] Error cases tested
- [ ] Examples in `examples/` directory

### Safety

- [ ] No `unwrap()` or `expect()` in library code
- [ ] Proper error propagation with `?`
- [ ] No `unsafe` unless documented and justified
- [ ] Lints configured appropriately
- [ ] Dependencies audited and minimal

---

## Examples

### Complete Client Example

```rust
//! Example API client demonstrating all patterns

use reqwest::{Client as HttpClient, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::sync::Arc;
use std::time::Duration;

/// Errors that can occur when using the Example client
#[derive(Error, Debug)]
pub enum ExampleError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Invalid configuration: {0}")]
    Config(String),
    
    #[error("Service error {status}: {message}")]
    Service { status: u16, message: String },
}

pub type Result<T> = std::result::Result<T, ExampleError>;

/// Configuration options for the Example client
#[derive(Clone, Debug)]
pub struct ExampleClientOptions {
    pub timeout: Duration,
    pub max_retries: u32,
}

impl Default for ExampleClientOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }
}

/// Client for the Example API
pub struct ExampleClient {
    client: HttpClient,
    base_url: Url,
    options: ExampleClientOptions,
}

impl ExampleClient {
    /// Creates a new Example client
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the API
    /// * `options` - Optional configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use example_client::{ExampleClient, ExampleClientOptions};
    ///
    /// let client = ExampleClient::new(
    ///     "https://api.example.com",
    ///     None
    /// ).unwrap();
    /// ```
    pub fn new<U>(
        base_url: U,
        options: Option<ExampleClientOptions>,
    ) -> Result<Self>
    where
        U: AsRef<str>,
    {
        let base_url = Url::parse(base_url.as_ref())
            .map_err(|e| ExampleError::Config(e.to_string()))?;
        
        let options = options.unwrap_or_default();
        
        let client = HttpClient::builder()
            .timeout(options.timeout)
            .build()?;
        
        Ok(Self {
            client,
            base_url,
            options,
        })
    }
    
    /// Gets a resource by ID
    pub async fn get_resource(&self, id: &str) -> Result<Resource> {
        let url = self.base_url
            .join(&format!("/resources/{}", id))
            .map_err(|e| ExampleError::Config(e.to_string()))?;
        
        let response = self.client
            .get(url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(ExampleError::Service {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }
        
        let resource = response.json().await?;
        Ok(resource)
    }
    
    /// Lists all resources with pagination
    pub async fn list_resources(&self) -> Result<Pager<Resource>> {
        // Implementation would return a Pager
        todo!()
    }
}

/// A resource in the Example API
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// A pager for paginated results
pub struct Pager<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Pager<T> {
    /// Collects all items into a vector
    pub async fn collect_all(self) -> Result<Vec<T>> {
        todo!()
    }
}
```

---

## References

- [Azure SDK Rust Guidelines](https://azure.github.io/azure-sdk/rust_introduction.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Elegant Library APIs in Rust](https://deterministic.space/elegant-apis-in-rust.html)

---

## Revision History

- 2025-01-20: Initial version combining Azure SDK, Design Patterns, and Elegant APIs standards
