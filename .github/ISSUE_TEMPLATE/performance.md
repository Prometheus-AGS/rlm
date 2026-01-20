---
name: Performance Issue
about: Report performance problems or regressions
title: '[PERF] '
labels: ['performance', 'needs-triage']
assignees: ''
---

## Performance Issue Type

- [ ] Performance regression
- [ ] Slow operation
- [ ] Memory leak
- [ ] High CPU usage
- [ ] High memory usage
- [ ] Scalability issue
- [ ] Request for optimization

## Environment

- **RLM Version**: <!-- e.g., v1.0.0 or commit hash -->
- **Rust Version**: <!-- Output of `rustc --version` -->
- **Operating System**: <!-- e.g., Ubuntu 22.04, macOS 13, Windows 11 -->
- **Hardware**: <!-- CPU, RAM, etc. -->
- **Deployment**: <!-- Docker, binary, etc. -->
- **Load**: <!-- Number of concurrent users, requests/second, etc. -->

## Problem Description

<!-- Describe the performance issue -->

## Performance Measurements

### Current Performance

<!-- Provide specific measurements -->

- **Response Time**: <!-- e.g., 2.5 seconds average -->
- **Throughput**: <!-- e.g., 100 requests/second -->
- **Memory Usage**: <!-- e.g., 2GB RSS -->
- **CPU Usage**: <!-- e.g., 80% average -->

### Expected Performance

<!-- What performance did you expect? -->

- **Response Time**: <!-- e.g., < 500ms -->
- **Throughput**: <!-- e.g., > 500 requests/second -->
- **Memory Usage**: <!-- e.g., < 1GB RSS -->
- **CPU Usage**: <!-- e.g., < 40% average -->

## Reproduction Steps

1. <!-- Step to reproduce the performance issue -->
2. <!-- Additional steps -->
3. <!-- Measurement method -->

## Test Configuration

<!-- Configuration that exhibits the performance issue -->

```yaml
# Paste relevant configuration
```

## Profiling Data

<!-- If you have profiling data, include it here -->

### CPU Profile

<!-- Results from CPU profiling tools -->

### Memory Profile

<!-- Results from memory profiling tools -->

### Benchmarks

```bash
# Commands used for benchmarking
cargo bench
```

<!-- Benchmark results -->

## Workload Characteristics

<!-- Describe the workload that causes the issue -->

- **Context Size**: <!-- e.g., 1MB average -->
- **Query Complexity**: <!-- e.g., simple aggregation -->
- **Concurrent Requests**: <!-- e.g., 50 -->
- **Request Pattern**: <!-- e.g., steady, bursty -->

## System Monitoring

<!-- If available, include system monitoring data -->

- **Disk I/O**: <!-- Usage patterns -->
- **Network I/O**: <!-- Usage patterns -->
- **Database Load**: <!-- If applicable -->
- **External API Calls**: <!-- Latency and volume -->

## Regression Information

<!-- If this is a regression -->

- **Last Known Good Version**: <!-- Version where performance was acceptable -->
- **First Bad Version**: <!-- Version where performance degraded -->
- **Suspected Changes**: <!-- PR or commit that might have caused regression -->

## Impact

- [ ] Affects development workflow
- [ ] Affects production workloads
- [ ] Makes feature unusable
- [ ] Causes timeouts/failures
- [ ] Affects user experience
- [ ] Increases costs

## Attempted Solutions

<!-- What have you tried to fix or work around the issue? -->

- [ ] Adjusted configuration
- [ ] Increased resources
- [ ] Code changes
- [ ] Different deployment method

## Additional Context

<!-- Any other relevant information -->

### Related Issues

<!-- Link to related performance issues -->

### External Dependencies

<!-- Performance issues with external services -->

## Analysis Request

<!-- What kind of analysis would be helpful? -->

- [ ] CPU profiling
- [ ] Memory profiling
- [ ] I/O analysis
- [ ] Algorithmic complexity review
- [ ] Database query optimization
- [ ] Caching strategy review

## Checklist

- [ ] I have provided specific performance measurements
- [ ] I have included reproduction steps
- [ ] I have tested with the latest version
- [ ] I have provided environment details
- [ ] I have searched for existing performance issues