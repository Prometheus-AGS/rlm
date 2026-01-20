# RLM OpenAI-Compatible Server Quickstart Guide

This guide demonstrates how to quickly set up and use the RLM server for long-context processing using the OpenAI-compatible API.

## Quick Setup

### 1. Install and Run Server

```bash
# Clone the repository
git clone https://github.com/example/rlm
cd rlm

# Build the server
cargo build --release --bin rlm-server

# Run with default configuration
./target/release/rlm-server

# Or run with custom configuration
./target/release/rlm-server \
  --port 8080 \
  --host 0.0.0.0 \
  --config config.yaml
```

### 2. Environment Configuration

Create a `.env` file or set environment variables:

```bash
# Required: LLM Provider Settings
RLM_LLM_PROVIDER=openai
RLM_LLM_API_KEY=sk-your-openai-api-key-here
RLM_LLM_MODEL=gpt-4

# Optional: Server Settings
RLM_SERVER_HOST=0.0.0.0
RLM_SERVER_PORT=8080

# Optional: Processing Limits
RLM_MAX_CONTEXT_SIZE=10000000  # 10M tokens
RLM_MAX_RECURSIVE_DEPTH=10
RLM_MAX_ITERATIONS=50

# Optional: Logging
RLM_LOG_LEVEL=info
RLM_LOG_FORMAT=json
```

### 3. YAML Configuration (Alternative)

Create `rlm-config.yaml`:

```yaml
server:
  host: "0.0.0.0"
  port: 8080

llm:
  provider: "openai"
  api_key: "sk-your-openai-api-key-here"  # Use env vars in production
  model: "gpt-4"
  base_url: "https://api.openai.com/v1"

rlm:
  max_context_size: 10000000
  max_recursive_depth: 10
  max_iterations: 50

logging:
  level: "info"
  format: "json"

# Azure OpenAI Configuration Example
# llm:
#   provider: "azure"
#   api_key: "your-azure-key"
#   base_url: "https://your-resource.openai.azure.com"
#   deployment_name: "gpt-4-deployment"
#   api_version: "2024-02-01"
```

## Basic Usage Examples

### Standard Chat Completion (Non-Streaming)

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "user",
        "content": "Hello, how are you?"
      }
    ],
    "max_tokens": 100
  }'
```

**Response:**
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! I'm doing well, thank you for asking. How can I assist you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 11,
    "completion_tokens": 17,
    "total_tokens": 28,
    "recursive_calls": 0
  }
}
```

### Long Context Processing (RLM Automatic)

```bash
# Example with a large document
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "user",
        "content": "Please analyze this large document and provide a summary: [5MB of text content here]"
      }
    ],
    "max_tokens": 500,
    "stream": false
  }'
```

**Note**: When context exceeds model limits, RLM automatically activates recursive processing.

### Streaming Response with Progress Events

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "user",
        "content": "Analyze this research paper and provide key insights: [large research paper text]"
      }
    ],
    "stream": true,
    "max_tokens": 1000
  }'
```

**Streaming Response Format:**
```
data: {"id":"rlm-event-1","object":"rlm.processing.chunk","created":1677652288,"data":{"type":"progress","phase":"context_loading","progress":0.1,"message":"Loading context chunks"}}

data: {"id":"rlm-event-2","object":"rlm.processing.chunk","created":1677652289,"data":{"type":"recursive_call","call_id":"call-abc","depth":1,"status":"started","prompt":"Analyze section 1..."}}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652290,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Based"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652291,"model":"gpt-4","choices":[{"index":0,"delta":{"content":" on"},"finish_reason":null}]}

data: [DONE]
```

## Python Client Example

### Using OpenAI SDK (Drop-in Replacement)

```python
import openai

# Configure to use RLM server instead of OpenAI
openai.api_base = "http://localhost:8080/v1"
openai.api_key = "your-api-key"

# Standard usage - RLM handles long contexts automatically
response = openai.ChatCompletion.create(
    model="gpt-4",
    messages=[
        {"role": "user", "content": "Analyze this large document..."}
    ],
    max_tokens=500
)

print(response.choices[0].message.content)
print(f"Recursive calls used: {response.usage.get('recursive_calls', 0)}")
```

### Streaming with Progress Monitoring

```python
import openai
import json

def handle_stream():
    stream = openai.ChatCompletion.create(
        model="gpt-4",
        messages=[
            {"role": "user", "content": "Large context analysis task..."}
        ],
        stream=True
    )

    for chunk in stream:
        if hasattr(chunk, 'object'):
            if chunk.object == "chat.completion.chunk":
                # Standard OpenAI streaming chunk
                if chunk.choices and chunk.choices[0].delta.get('content'):
                    print(chunk.choices[0].delta.content, end='')

            elif chunk.object == "rlm.processing.chunk":
                # RLM-specific progress event
                event_data = chunk.data
                if event_data['type'] == 'progress':
                    progress = event_data['progress'] * 100
                    print(f"\nProgress: {progress:.1f}% - {event_data['message']}")
                elif event_data['type'] == 'recursive_call':
                    print(f"\nRecursive call at depth {event_data['depth']}")

handle_stream()
```

## Node.js Client Example

```javascript
import fetch from 'node-fetch';

async function longContextCompletion() {
    const response = await fetch('http://localhost:8080/v1/chat/completions', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer your-api-key'
        },
        body: JSON.stringify({
            model: 'gpt-4',
            messages: [
                {
                    role: 'user',
                    content: 'Analyze this massive dataset and provide insights: [large content]'
                }
            ],
            stream: false,
            max_tokens: 1000
        })
    });

    const result = await response.json();
    console.log('Response:', result.choices[0].message.content);
    console.log('Recursive calls:', result.usage.recursive_calls);
    console.log('Total tokens:', result.usage.total_tokens);
}

async function streamingCompletion() {
    const response = await fetch('http://localhost:8080/v1/chat/completions', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer your-api-key'
        },
        body: JSON.stringify({
            model: 'gpt-4',
            messages: [{ role: 'user', content: 'Large context task...' }],
            stream: true
        })
    });

    const reader = response.body.getReader();
    const decoder = new TextDecoder();

    while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const chunk = decoder.decode(value);
        const lines = chunk.split('\n').filter(line => line.startsWith('data: '));

        for (const line of lines) {
            if (line === 'data: [DONE]') return;

            try {
                const data = JSON.parse(line.substring(6));

                if (data.object === 'chat.completion.chunk') {
                    // Standard streaming content
                    const content = data.choices?.[0]?.delta?.content;
                    if (content) process.stdout.write(content);
                } else if (data.object === 'rlm.processing.chunk') {
                    // RLM progress events
                    const event = data.data;
                    if (event.type === 'progress') {
                        console.log(`\nProgress: ${(event.progress * 100).toFixed(1)}% - ${event.message}`);
                    }
                }
            } catch (e) {
                // Skip malformed chunks
            }
        }
    }
}

// Usage
longContextCompletion();
// streamingCompletion();
```

## Advanced Configuration

### RLM-Specific Parameters

When you need fine-grained control over RLM processing:

```json
{
  "model": "gpt-4",
  "messages": [...],
  "stream": true,
  "rlm_config": {
    "max_recursive_depth": 5,
    "context_chunk_size": 25000,
    "enable_progress_events": true,
    "aggregation_strategy": "parallel"
  }
}
```

### Multi-Backend Setup

Configure multiple LLM providers for load balancing:

```yaml
llm:
  primary:
    provider: "openai"
    api_key: "sk-openai-key"
    model: "gpt-4"

  fallback:
    provider: "azure"
    api_key: "azure-key"
    base_url: "https://resource.openai.azure.com"
    deployment_name: "gpt-4-deployment"

  load_balancing:
    strategy: "round_robin"  # or "least_latency", "random"
    health_check_interval: 30
```

## Health Monitoring

### Health Check Endpoint

```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-19T12:00:00Z",
  "version": "1.0.0",
  "backends": {
    "openai": {
      "status": "healthy",
      "response_time_ms": 150
    }
  },
  "resources": {
    "memory_usage_mb": 256,
    "cpu_usage_percent": 15,
    "active_sessions": 3
  }
}
```

### Metrics Endpoint

```bash
curl http://localhost:8080/metrics
```

**Prometheus Metrics Output:**
```
# HELP rlm_requests_total Total RLM requests processed
# TYPE rlm_requests_total counter
rlm_requests_total{model="gpt-4",status="success"} 1234

# HELP rlm_recursive_depth_histogram Distribution of recursive call depths
# TYPE rlm_recursive_depth_histogram histogram
rlm_recursive_depth_histogram_bucket{le="1"} 500
rlm_recursive_depth_histogram_bucket{le="5"} 1200
```

## Common Use Cases

### 1. Document Analysis
Perfect for analyzing large documents, research papers, legal contracts:

```python
# Analyze a 100-page legal document
response = openai.ChatCompletion.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": f"Analyze this contract for key terms and risks: {contract_text}"
    }],
    max_tokens=2000
)
```

### 2. Code Repository Understanding
Understand large codebases by providing entire file contents:

```python
# Analyze entire codebase structure
response = openai.ChatCompletion.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": f"Explain the architecture of this codebase: {all_source_files}"
    }]
)
```

### 3. Research Paper Synthesis
Synthesize information from multiple research papers:

```python
# Combine insights from multiple papers
response = openai.ChatCompletion.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": f"Compare and synthesize findings from these papers: {paper1 + paper2 + paper3}"
    }],
    stream=True  # Use streaming for long processing times
)
```

## Troubleshooting

### Common Issues

1. **Context Too Large Error**
   - Increase `RLM_MAX_CONTEXT_SIZE` in configuration
   - Use streaming responses for better progress tracking

2. **Recursive Depth Exceeded**
   - Increase `RLM_MAX_RECURSIVE_DEPTH`
   - Simplify the query to require less decomposition

3. **Backend Timeout**
   - Configure longer timeout in backend settings
   - Use fallback backend configuration

4. **Memory Usage High**
   - Reduce `context_chunk_size` in RLM config
   - Enable garbage collection more frequently

### Debug Logging

Enable debug logging for detailed processing information:

```bash
RLM_LOG_LEVEL=debug ./target/release/rlm-server
```

### Support

For issues, feature requests, or questions:
- GitHub Issues: https://github.com/example/rlm/issues
- Documentation: https://rlm-docs.example.com
- API Reference: https://rlm-api.example.com/docs

This quickstart guide covers the essential functionality for getting started with RLM's long-context processing capabilities. The server provides full OpenAI API compatibility while automatically handling contexts that exceed traditional model limits through intelligent recursive decomposition.