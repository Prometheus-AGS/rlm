//! API Usage Examples
//!
//! This module provides comprehensive examples for using the RLM server API,
//! including code samples in multiple programming languages and common usage patterns.

use serde_json::{json, Value};

/// Generate comprehensive API usage examples
pub fn generate_examples() -> Value {
    json!({
        "curl_examples": generate_curl_examples(),
        "javascript_examples": generate_javascript_examples(),
        "python_examples": generate_python_examples(),
        "rust_examples": generate_rust_examples(),
        "integration_patterns": generate_integration_patterns(),
        "use_cases": generate_use_cases()
    })
}

/// Generate cURL command examples
fn generate_curl_examples() -> Value {
    json!({
        "basic_completion": {
            "description": "Simple chat completion request",
            "command": r#"curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "user",
        "content": "What is the capital of France?"
      }
    ],
    "max_tokens": 100,
    "temperature": 0.7
  }'"#
        },
        "long_context_completion": {
            "description": "Long context completion with RLM processing",
            "command": r#"curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "user",
        "content": "Please analyze this 500-page research document: [VERY LONG DOCUMENT CONTENT...] What are the key findings?"
      }
    ],
    "max_tokens": 2000,
    "temperature": 0.3,
    "rlm_config": {
      "max_recursion_depth": 5,
      "chunk_strategy": "semantic",
      "enable_streaming": true
    }
  }'"#
        },
        "streaming_completion": {
            "description": "Streaming response with Server-Sent Events",
            "command": r#"curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -H "Accept: text/event-stream" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "user",
        "content": "Write a detailed explanation of quantum computing."
      }
    ],
    "stream": true,
    "max_tokens": 1500
  }'"#
        },
        "health_check": {
            "description": "Server health check",
            "command": "curl -X GET http://localhost:8080/health"
        },
        "list_models": {
            "description": "List available models",
            "command": "curl -X GET http://localhost:8080/v1/models -H \"Authorization: Bearer your-api-key\""
        },
        "get_metrics": {
            "description": "Get Prometheus metrics",
            "command": "curl -X GET http://localhost:8080/metrics"
        }
    })
}

/// Generate JavaScript/TypeScript examples
fn generate_javascript_examples() -> Value {
    json!({
        "basic_client": {
            "description": "Basic RLM client setup in JavaScript",
            "code": r#"// npm install node-fetch
import fetch from 'node-fetch';

class RLMClient {
  constructor(apiKey, baseURL = 'http://localhost:8080') {
    this.apiKey = apiKey;
    this.baseURL = baseURL;
  }

  async chatCompletion(messages, options = {}) {
    const response = await fetch(`${this.baseURL}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
      },
      body: JSON.stringify({
        model: 'gpt-4',
        messages: messages,
        ...options
      }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    return response.json();
  }

  async streamCompletion(messages, options = {}) {
    const response = await fetch(`${this.baseURL}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
        'Accept': 'text/event-stream',
      },
      body: JSON.stringify({
        model: 'gpt-4',
        messages: messages,
        stream: true,
        ...options
      }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    return response.body;
  }
}

// Usage example
const client = new RLMClient('your-api-key');

// Basic completion
const response = await client.chatCompletion([
  { role: 'user', content: 'What is the capital of France?' }
]);
console.log(response.choices[0].message.content);

// Long context with RLM
const longContextResponse = await client.chatCompletion([
  { role: 'user', content: 'Analyze this large document: [DOCUMENT...]' }
], {
  max_tokens: 2000,
  rlm_config: {
    max_recursion_depth: 5,
    chunk_strategy: 'semantic'
  }
});

// Streaming completion
const stream = await client.streamCompletion([
  { role: 'user', content: 'Write a story about space exploration.' }
]);

const reader = stream.getReader();
const decoder = new TextDecoder();

try {
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    const chunk = decoder.decode(value);
    const lines = chunk.split('\n');

    for (const line of lines) {
      if (line.startsWith('data: ')) {
        const data = line.slice(6);
        if (data === '[DONE]') {
          console.log('Stream complete');
          break;
        }

        try {
          const parsed = JSON.parse(data);
          const content = parsed.choices[0]?.delta?.content;
          if (content) {
            process.stdout.write(content);
          }
        } catch (e) {
          // Skip invalid JSON
        }
      }
    }
  }
} finally {
  reader.releaseLock();
}"#
        },
        "typescript_client": {
            "description": "TypeScript client with types",
            "code": r#"interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
  name?: string;
}

interface RLMConfig {
  max_recursion_depth?: number;
  chunk_strategy?: 'fixed_size' | 'semantic' | 'adaptive';
  enable_streaming?: boolean;
  context_window?: number;
  aggregation_strategy?: 'simple' | 'weighted' | 'hierarchical';
}

interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  max_tokens?: number;
  temperature?: number;
  top_p?: number;
  stream?: boolean;
  stop?: string | string[];
  presence_penalty?: number;
  frequency_penalty?: number;
  rlm_config?: RLMConfig;
}

interface Usage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

interface RLMMetadata {
  recursive_calls: number;
  context_chunks: number;
  processing_time_ms: number;
  memory_usage_mb: number;
  complexity_class: 'O(1)' | 'O(log n)' | 'O(n)' | 'O(n log n)' | 'O(n^2)';
}

interface ChatCompletionResponse {
  id: string;
  object: 'chat.completion';
  created: number;
  model: string;
  choices: Array<{
    index: number;
    message: ChatMessage;
    finish_reason: string;
  }>;
  usage: Usage;
  rlm_metadata?: RLMMetadata;
}

class TypedRLMClient {
  constructor(
    private apiKey: string,
    private baseURL: string = 'http://localhost:8080'
  ) {}

  async chatCompletion(
    request: ChatCompletionRequest
  ): Promise<ChatCompletionResponse> {
    const response = await fetch(`${this.baseURL}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`API Error: ${error.error?.message || response.statusText}`);
    }

    return response.json();
  }

  async *streamCompletion(
    request: ChatCompletionRequest
  ): AsyncGenerator<any, void, unknown> {
    const response = await fetch(`${this.baseURL}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
        'Accept': 'text/event-stream',
      },
      body: JSON.stringify({ ...request, stream: true }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const reader = response.body!.getReader();
    const decoder = new TextDecoder();

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const chunk = decoder.decode(value);
        const lines = chunk.split('\n');

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data === '[DONE]') return;

            try {
              yield JSON.parse(data);
            } catch {
              // Skip invalid JSON
            }
          }
        }
      }
    } finally {
      reader.releaseLock();
    }
  }
}

// Usage
const client = new TypedRLMClient('your-api-key');

const response = await client.chatCompletion({
  model: 'gpt-4',
  messages: [{ role: 'user', content: 'Hello!' }],
  max_tokens: 100,
  rlm_config: {
    max_recursion_depth: 3,
    chunk_strategy: 'adaptive'
  }
});

console.log(response.choices[0].message.content);
console.log(`Used ${response.usage.total_tokens} tokens`);
console.log(`Made ${response.rlm_metadata?.recursive_calls} recursive calls`);"#
        }
    })
}

/// Generate Python examples
fn generate_python_examples() -> Value {
    json!({
        "basic_client": {
            "description": "Python client using requests library",
            "code": r#"import requests
import json
import sseclient
from typing import List, Dict, Optional, Iterator

class RLMClient:
    def __init__(self, api_key: str, base_url: str = "http://localhost:8080"):
        self.api_key = api_key
        self.base_url = base_url
        self.session = requests.Session()
        self.session.headers.update({
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json"
        })

    def chat_completion(self, messages: List[Dict], **kwargs) -> Dict:
        """Create a chat completion request."""
        payload = {
            "model": "gpt-4",
            "messages": messages,
            **kwargs
        }

        response = self.session.post(
            f"{self.base_url}/v1/chat/completions",
            json=payload
        )
        response.raise_for_status()
        return response.json()

    def stream_completion(self, messages: List[Dict], **kwargs) -> Iterator[Dict]:
        """Create a streaming chat completion request."""
        payload = {
            "model": "gpt-4",
            "messages": messages,
            "stream": True,
            **kwargs
        }

        response = self.session.post(
            f"{self.base_url}/v1/chat/completions",
            json=payload,
            headers={"Accept": "text/event-stream"},
            stream=True
        )
        response.raise_for_status()

        client = sseclient.SSEClient(response)
        for event in client.events():
            if event.data == "[DONE]":
                break
            try:
                yield json.loads(event.data)
            except json.JSONDecodeError:
                continue

    def list_models(self) -> Dict:
        """List available models."""
        response = self.session.get(f"{self.base_url}/v1/models")
        response.raise_for_status()
        return response.json()

    def health_check(self) -> Dict:
        """Check server health."""
        response = self.session.get(f"{self.base_url}/health")
        response.raise_for_status()
        return response.json()

# Usage examples
client = RLMClient("your-api-key")

# Basic completion
response = client.chat_completion([
    {"role": "user", "content": "What is the capital of France?"}
])
print(response["choices"][0]["message"]["content"])

# Long context with RLM configuration
long_context_response = client.chat_completion(
    messages=[{
        "role": "user",
        "content": "Analyze this large document: [DOCUMENT CONTENT...]"
    }],
    max_tokens=2000,
    temperature=0.3,
    rlm_config={
        "max_recursion_depth": 5,
        "chunk_strategy": "semantic",
        "enable_streaming": True
    }
)

print(f"Recursive calls made: {long_context_response.get('rlm_metadata', {}).get('recursive_calls', 0)}")
print(f"Processing time: {long_context_response.get('rlm_metadata', {}).get('processing_time_ms', 0)}ms")

# Streaming completion
print("Streaming response:")
for chunk in client.stream_completion([
    {"role": "user", "content": "Write a story about space exploration."}
]):
    content = chunk.get("choices", [{}])[0].get("delta", {}).get("content", "")
    if content:
        print(content, end="", flush=True)

print("\nStream complete!")

# Check available models
models = client.list_models()
print(f"Available models: {[model['id'] for model in models['data']]}")

# Health check
health = client.health_check()
print(f"Server status: {health['status']}")
print(f"Uptime: {health['uptime_seconds']}s")"#
        },
        "async_client": {
            "description": "Asynchronous Python client using aiohttp",
            "code": r#"import aiohttp
import asyncio
import json
from typing import List, Dict, Optional, AsyncIterator

class AsyncRLMClient:
    def __init__(self, api_key: str, base_url: str = "http://localhost:8080"):
        self.api_key = api_key
        self.base_url = base_url
        self.headers = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json"
        }

    async def chat_completion(self, messages: List[Dict], **kwargs) -> Dict:
        """Create a chat completion request."""
        payload = {
            "model": "gpt-4",
            "messages": messages,
            **kwargs
        }

        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{self.base_url}/v1/chat/completions",
                json=payload,
                headers=self.headers
            ) as response:
                response.raise_for_status()
                return await response.json()

    async def stream_completion(self, messages: List[Dict], **kwargs) -> AsyncIterator[Dict]:
        """Create a streaming chat completion request."""
        payload = {
            "model": "gpt-4",
            "messages": messages,
            "stream": True,
            **kwargs
        }

        headers = {**self.headers, "Accept": "text/event-stream"}

        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{self.base_url}/v1/chat/completions",
                json=payload,
                headers=headers
            ) as response:
                response.raise_for_status()

                async for line in response.content:
                    line = line.decode('utf-8').strip()
                    if line.startswith('data: '):
                        data = line[6:]
                        if data == "[DONE]":
                            break
                        try:
                            yield json.loads(data)
                        except json.JSONDecodeError:
                            continue

# Usage examples
async def main():
    client = AsyncRLMClient("your-api-key")

    # Basic completion
    response = await client.chat_completion([
        {"role": "user", "content": "What is quantum computing?"}
    ])
    print(response["choices"][0]["message"]["content"])

    # Streaming completion
    print("Streaming response:")
    async for chunk in client.stream_completion([
        {"role": "user", "content": "Explain machine learning step by step."}
    ]):
        content = chunk.get("choices", [{}])[0].get("delta", {}).get("content", "")
        if content:
            print(content, end="", flush=True)

    print("\nDone!")

# Run the async example
asyncio.run(main())"#
        }
    })
}

/// Generate Rust examples
fn generate_rust_examples() -> Value {
    json!({
        "basic_client": {
            "description": "Rust client using reqwest",
            "code": r#"use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio_stream::StreamExt;

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RLMConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_recursion_depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_streaming: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rlm_config: Option<RLMConfig>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    choices: Vec<Choice>,
    usage: Usage,
    rlm_metadata: Option<RLMMetadata>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct RLMMetadata {
    recursive_calls: u32,
    processing_time_ms: u64,
    complexity_class: String,
}

pub struct RLMClient {
    client: Client,
    base_url: String,
}

impl RLMClient {
    pub fn new(api_key: &str, base_url: Option<&str>) -> Result<Self, reqwest::Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.unwrap_or("http://localhost:8080").to_string(),
        })
    }

    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(&format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error_text = response.text().await?;
            Err(format!("API error: {}", error_text).into())
        }
    }

    pub async fn stream_completion(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<impl tokio_stream::Stream<Item = Result<Value, Box<dyn std::error::Error>>>, Box<dyn std::error::Error>> {
        request.stream = Some(true);

        let response = self
            .client
            .post(&format!("{}/v1/chat/completions", self.base_url))
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("API error: {}", error_text).into());
        }

        let stream = response.bytes_stream();
        Ok(stream.map(|chunk| {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                break;
                            }
                            if let Ok(json) = serde_json::from_str::<Value>(data) {
                                return Ok(json);
                            }
                        }
                    }
                    // Return empty if no valid data found
                    Ok(serde_json::Value::Null)
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RLMClient::new("your-api-key", None)?;

    // Basic completion
    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "What is the capital of France?".to_string(),
        }],
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: None,
        rlm_config: None,
    };

    let response = client.chat_completion(request).await?;
    println!("Response: {}", response.choices[0].message.content);
    println!("Tokens used: {}", response.usage.total_tokens);

    // Long context with RLM
    let long_context_request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Analyze this large document: [DOCUMENT...]".to_string(),
        }],
        max_tokens: Some(2000),
        temperature: Some(0.3),
        stream: None,
        rlm_config: Some(RLMConfig {
            max_recursion_depth: Some(5),
            chunk_strategy: Some("semantic".to_string()),
            enable_streaming: Some(true),
        }),
    };

    let response = client.chat_completion(long_context_request).await?;
    if let Some(metadata) = response.rlm_metadata {
        println!("Recursive calls: {}", metadata.recursive_calls);
        println!("Processing time: {}ms", metadata.processing_time_ms);
        println!("Complexity class: {}", metadata.complexity_class);
    }

    // Streaming example
    println!("Streaming response:");
    let streaming_request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Write a short story about space exploration.".to_string(),
        }],
        max_tokens: Some(500),
        temperature: Some(0.8),
        stream: Some(true),
        rlm_config: None,
    };

    let mut stream = client.stream_completion(streaming_request).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(data) => {
                if let Some(content) = data["choices"][0]["delta"]["content"].as_str() {
                    print!("{}", content);
                }
            }
            Err(e) => eprintln!("Stream error: {}", e),
        }
    }
    println!("\nStream complete!");

    Ok(())
}"#
        }
    })
}

/// Generate integration patterns
fn generate_integration_patterns() -> Value {
    json!({
        "openai_drop_in": {
            "description": "Drop-in replacement for OpenAI client",
            "example": "Simply change the base URL from api.openai.com to your RLM server, and existing OpenAI client code will work with RLM enhancements for long contexts."
        },
        "langchain_integration": {
            "description": "Integration with LangChain framework",
            "code": r#"from langchain.llms import OpenAI
from langchain.schema import BaseLanguageModel

class RLMLanguageModel(BaseLanguageModel):
    def __init__(self, api_key: str, base_url: str = "http://localhost:8080"):
        self.api_key = api_key
        self.base_url = base_url

    def _call(self, prompt: str, stop: Optional[List[str]] = None) -> str:
        # Implement RLM API call
        import requests

        response = requests.post(
            f"{self.base_url}/v1/chat/completions",
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "Content-Type": "application/json"
            },
            json={
                "model": "gpt-4",
                "messages": [{"role": "user", "content": prompt}],
                "stop": stop,
                "rlm_config": {
                    "max_recursion_depth": 5,
                    "chunk_strategy": "adaptive"
                }
            }
        )

        return response.json()["choices"][0]["message"]["content"]

# Usage with LangChain
rlm = RLMLanguageModel("your-api-key")
result = rlm("Analyze this very long document: [CONTENT...]")
print(result)"#
        },
        "load_balancer_config": {
            "description": "NGINX configuration for load balancing RLM servers",
            "config": r#"upstream rlm_servers {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;

    # Health checks
    check interval=3000 rise=2 fall=5 timeout=1000;
}

server {
    listen 80;
    server_name rlm.example.com;

    # Increase timeout for long-running requests
    proxy_read_timeout 600s;
    proxy_connect_timeout 60s;
    proxy_send_timeout 600s;

    # Increase buffer sizes for large contexts
    client_max_body_size 100M;
    proxy_buffer_size 64k;
    proxy_buffers 8 64k;

    location / {
        proxy_pass http://rlm_servers;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Enable streaming
        proxy_buffering off;
        proxy_cache off;
    }

    # Health check endpoint
    location /health {
        access_log off;
        proxy_pass http://rlm_servers/health;
    }

    # Metrics endpoint (restrict access)
    location /metrics {
        allow 10.0.0.0/8;
        deny all;
        proxy_pass http://rlm_servers/metrics;
    }
}"#
        },
        "docker_compose": {
            "description": "Docker Compose setup for RLM server cluster",
            "yaml": r#"version: '3.8'

services:
  rlm-server-1:
    image: rlm/server:latest
    ports:
      - "8080:8080"
    environment:
      - RLM_PORT=8080
      - RLM_HOST=0.0.0.0
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - RLM_LOG_LEVEL=info
    volumes:
      - ./config.yaml:/app/config.yaml:ro
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  rlm-server-2:
    image: rlm/server:latest
    ports:
      - "8081:8080"
    environment:
      - RLM_PORT=8080
      - RLM_HOST=0.0.0.0
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - RLM_LOG_LEVEL=info
    volumes:
      - ./config.yaml:/app/config.yaml:ro
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - rlm-server-1
      - rlm-server-2

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-storage:/var/lib/grafana
      - ./grafana-dashboards:/etc/grafana/provisioning/dashboards:ro

volumes:
  grafana-storage:"#
        }
    })
}

/// Generate use cases and examples
fn generate_use_cases() -> Value {
    json!({
        "document_analysis": {
            "title": "Large Document Analysis",
            "description": "Analyze research papers, legal documents, or technical manuals that exceed typical model context limits",
            "example": "A 500-page research paper analysis where RLM breaks down the document into semantic chunks, processes each section recursively, and provides comprehensive insights.",
            "code_snippet": r#"const response = await client.chatCompletion([
  {
    role: 'user',
    content: `Please analyze this comprehensive research paper: ${largeDocumentContent}

    Provide:
    1. Executive summary
    2. Key findings and conclusions
    3. Methodology assessment
    4. Limitations and future work
    5. Relevance to current field developments`
  }
], {
  max_tokens: 3000,
  rlm_config: {
    max_recursion_depth: 6,
    chunk_strategy: 'semantic',
    aggregation_strategy: 'hierarchical'
  }
});"#
        },
        "code_review": {
            "title": "Large Codebase Review",
            "description": "Review entire codebases or large code changes that span multiple files and modules",
            "example": "Review a full-stack application codebase for security vulnerabilities, performance issues, and code quality.",
            "code_snippet": r#"const codeReview = await client.chatCompletion([
  {
    role: 'user',
    content: `Review this codebase for security issues and performance problems:

${fullCodebase}

Focus on:
1. SQL injection vulnerabilities
2. XSS prevention
3. Authentication/authorization flows
4. Performance bottlenecks
5. Code organization and maintainability`
  }
], {
  max_tokens: 2500,
  rlm_config: {
    max_recursion_depth: 5,
    chunk_strategy: 'adaptive'
  }
});"#
        },
        "data_analysis": {
            "title": "Large Dataset Analysis",
            "description": "Analyze datasets that are too large to fit in a single context window",
            "example": "Process a year's worth of sales data, customer feedback, or system logs to identify patterns and trends.",
            "code_snippet": r#"# Python example for data analysis
response = client.chat_completion([
    {
        "role": "user",
        "content": f"""Analyze this sales dataset: {large_dataset}

        Please provide:
        1. Sales trends over time
        2. Top performing products/categories
        3. Customer behavior patterns
        4. Seasonal variations
        5. Recommendations for optimization"""
    }
],
max_tokens=2000,
rlm_config={
    "max_recursion_depth": 5,
    "chunk_strategy": "semantic",
    "aggregation_strategy": "weighted"
})"#
        },
        "multi_document_qa": {
            "title": "Multi-Document Question Answering",
            "description": "Answer questions that require information from multiple large documents",
            "example": "Compare and contrast multiple research papers, legal cases, or technical specifications.",
            "code_snippet": r#"curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "user",
        "content": "Compare the methodologies and findings across these research papers: [PAPER1] ... [PAPER2] ... [PAPER3] ... What are the common themes and where do they disagree?"
      }
    ],
    "max_tokens": 2500,
    "rlm_config": {
      "max_recursion_depth": 7,
      "chunk_strategy": "adaptive",
      "aggregation_strategy": "hierarchical"
    }
  }'"#
        },
        "content_generation": {
            "title": "Long-Form Content Generation",
            "description": "Generate comprehensive content based on extensive source material",
            "example": "Create detailed technical documentation, comprehensive guides, or research reports based on multiple sources.",
            "code_snippet": r#"const technicalGuide = await client.chatCompletion([
  {
    role: 'user',
    content: `Based on these technical specifications and documentation: ${technicalDocs}

    Create a comprehensive implementation guide that includes:
    1. Overview and architecture
    2. Step-by-step setup instructions
    3. Configuration examples
    4. Best practices and common pitfalls
    5. Troubleshooting guide
    6. Performance optimization tips`
  }
], {
  max_tokens: 4000,
  temperature: 0.3,
  rlm_config: {
    max_recursion_depth: 6,
    chunk_strategy: 'semantic',
    enable_streaming: true
  }
});"#
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_generation() {
        let examples = generate_examples();

        // Verify structure
        assert!(examples["curl_examples"].is_object());
        assert!(examples["javascript_examples"].is_object());
        assert!(examples["python_examples"].is_object());
        assert!(examples["rust_examples"].is_object());
        assert!(examples["integration_patterns"].is_object());
        assert!(examples["use_cases"].is_object());
    }

    #[test]
    fn test_curl_examples() {
        let curl_examples = generate_curl_examples();

        assert!(curl_examples["basic_completion"]["command"].is_string());
        assert!(curl_examples["streaming_completion"]["command"].is_string());
        assert!(curl_examples["health_check"]["command"].is_string());

        // Verify commands contain expected elements
        let basic_cmd = curl_examples["basic_completion"]["command"].as_str().unwrap();
        assert!(basic_cmd.contains("/v1/chat/completions"));
        assert!(basic_cmd.contains("Authorization: Bearer"));
    }

    #[test]
    fn test_use_cases() {
        let use_cases = generate_use_cases();

        assert!(use_cases["document_analysis"]["title"].is_string());
        assert!(use_cases["code_review"]["description"].is_string());
        assert!(use_cases["data_analysis"]["code_snippet"].is_string());

        // Verify use cases have required fields
        for (_key, case) in use_cases.as_object().unwrap() {
            assert!(case["title"].is_string());
            assert!(case["description"].is_string());
            assert!(case["code_snippet"].is_string());
        }
    }
}