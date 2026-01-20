# Tasks: RLM OpenAI-Compatible Server

**Input**: Design documents from `/specs/001-rlm-openai-server/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Integration tests are explicitly required by the specification and paper benchmarks validation. All test tasks are mandatory for this implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story, following the hexagonal architecture pattern.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md structure - Cargo workspace with hexagonal architecture:
- **Core crate**: `crates/rlm-core/src/`
- **REPL crate**: `crates/rlm-repl-rhai/src/`
- **Server crate**: `crates/rlm-server/src/`
- **Tests**: `crates/*/tests/` and workspace-level `tests/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and Cargo workspace structure

- [X] T001 Create Cargo workspace structure per implementation plan at repository root
- [X] T002 [P] Initialize rlm-core crate with Cargo.toml dependencies (thiserror, serde, tokio, tracing)
- [X] T003 [P] Initialize rlm-repl-rhai crate with Cargo.toml dependencies (rhai, rlm-core, tokio)
- [X] T004 [P] Initialize rlm-server crate with Cargo.toml dependencies (axum, rlm-core, clap, figment)
- [X] T005 [P] Configure workspace-level Cargo.toml with member crates and shared dependencies
- [X] T006 [P] Setup clippy and rustfmt configuration for unsafe_code forbidding and Microsoft coding standards
- [X] T007 [P] Create workspace-level tests directory structure for golden fixtures
- [X] T008 [P] Setup examples directory with basic usage demonstrations

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T009 Create core domain types in crates/rlm-core/src/types.rs (RlmRequest, RlmEvent, RlmResponse)
- [X] T010 [P] Implement structured error types with thiserror in crates/rlm-core/src/error.rs
- [X] T011 [P] Define ReplBackend port trait in crates/rlm-core/src/ports/repl_backend.rs
- [X] T012 [P] Define LlmProvider port trait in crates/rlm-core/src/ports/llm_provider.rs
- [X] T013 [P] Define EventSink port trait in crates/rlm-core/src/ports/event_sink.rs
- [X] T014 Create ports module structure in crates/rlm-core/src/ports/mod.rs
- [X] T015 Setup lib.rs with module exports in crates/rlm-core/src/lib.rs
- [X] T016 [P] Configure tracing subscriber infrastructure in crates/rlm-server/src/logging.rs
- [X] T017 [P] Create configuration management structure in crates/rlm-server/src/config/mod.rs
- [X] T018 [P] Implement environment variable configuration in crates/rlm-server/src/config/env.rs
- [X] T019 [P] Implement YAML configuration loading in crates/rlm-server/src/config/yaml.rs
- [X] T020 [P] Implement CLI argument parsing in crates/rlm-server/src/config/cli.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Basic Long-Context Chat Completion (Priority: P1) 🎯 MVP

**Goal**: AI application developers can send long-context prompts (>1M tokens) to RLM server and receive accurate responses through recursive decomposition

**Independent Test**: Send 2M token document with simple question via POST to /v1/chat/completions and verify accurate response demonstrating full document comprehension

### Integration Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T021 [P] [US1] Create OpenAI API contract test for /v1/chat/completions in crates/rlm-server/tests/contract/test_openai_api.rs
- [X] T022 [P] [US1] Create RLM core executor unit tests in crates/rlm-core/tests/executor_tests.rs
- [X] T023 [P] [US1] Create long-context integration test in tests/integration/test_long_context.rs
- [X] T024 [P] [US1] Setup golden test fixtures for S-NIAH dataset in tests/fixtures/s_niah/

### Core RLM Implementation for User Story 1

- [X] T025 [P] [US1] Implement RlmSession entity in crates/rlm-core/src/session.rs
- [X] T026 [P] [US1] Implement RecursiveCallTree entity in crates/rlm-core/src/recursive_call.rs
- [X] T027 [US1] Create core RLM executor in crates/rlm-core/src/executor.rs (depends on T025, T026)
- [X] T028 [US1] Implement context analysis and chunking logic in crates/rlm-core/src/context_analyzer.rs
- [X] T029 [P] [US1] Implement RhaiReplBackend in crates/rlm-repl-rhai/src/backend.rs
- [X] T030 [P] [US1] Configure Rhai engine with safety limits in crates/rlm-repl-rhai/src/engine.rs
- [X] T031 [US1] Implement basic llm_query() function for REPL in crates/rlm-repl-rhai/src/functions.rs
- [X] T032 [US1] Create OpenAI LLM provider adapter in crates/rlm-server/src/adapters/openai.rs
- [X] T033 [US1] Implement chat completions endpoint in crates/rlm-server/src/server/routes.rs
- [X] T034 [US1] Add request validation and error handling for OpenAI API compatibility
- [X] T035 [US1] Integrate RLM executor with HTTP endpoint handler

**Checkpoint**: At this point, User Story 1 should be fully functional - basic long-context completion works independently

---

## Phase 4: User Story 2 - Streaming Response Support (Priority: P1)

**Goal**: Enable real-time streaming responses from RLM processing with Server-Sent Events for responsive user experience

**Independent Test**: Send request with "stream": true and verify proper SSE headers, incremental data chunks, progress events, and [DONE] marker

### Integration Tests for User Story 2

- [X] T036 [P] [US2] Create SSE streaming contract test in crates/rlm-server/tests/contract/test_streaming.rs
- [X] T037 [P] [US2] Create RLM event streaming integration test in tests/integration/test_streaming.rs

### Streaming Implementation for User Story 2

- [X] T038 [P] [US2] Implement SSE event sink adapter in crates/rlm-server/src/adapters/sse_sink.rs
- [X] T039 [P] [US2] Create streaming response handler in crates/rlm-server/src/server/streaming.rs
- [X] T040 [US2] Extend RLM executor to support event streaming in crates/rlm-core/src/executor.rs
- [X] T041 [US2] Implement RLM-specific event types in crates/rlm-core/src/events.rs
- [X] T042 [US2] Add streaming support to chat completions endpoint in crates/rlm-server/src/server/routes.rs
- [X] T043 [US2] Implement progress tracking for recursive calls in crates/rlm-core/src/progress.rs
- [X] T044 [US2] Add proper SSE connection management and cleanup

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - both non-streaming and streaming completions

---

## Phase 5: User Story 3 - Multi-Backend Configuration (Priority: P2)

**Goal**: System administrators can configure different OpenAI-compatible backends (OpenAI, Azure, local models) through multiple configuration methods

**Independent Test**: Configure Azure OpenAI backend and verify all sub-LLM calls route correctly with proper authentication

### Integration Tests for User Story 3

- [X] T045 [P] [US3] Create multi-backend configuration test in crates/rlm-server/tests/integration/test_backends.rs
- [X] T046 [P] [US3] Create Azure OpenAI adapter test in crates/rlm-server/tests/contract/test_azure.rs

### Multi-Backend Implementation for User Story 3

- [X] T047 [P] [US3] Implement Azure OpenAI provider adapter in crates/rlm-server/src/adapters/azure.rs
- [X] T048 [P] [US3] Create backend configuration entity in crates/rlm-core/src/backend_config.rs
- [X] T049 [US3] Implement backend factory pattern in crates/rlm-server/src/backend_factory.rs
- [X] T050 [US3] Extend configuration management for backend selection in crates/rlm-server/src/config/backends.rs
- [X] T051 [US3] Add configuration hierarchy validation (CLI > env > YAML) in crates/rlm-server/src/config/validator.rs
- [X] T052 [US3] Implement backend health checking in crates/rlm-server/src/health/backend_check.rs
- [X] T053 [US3] Add backend routing logic to RLM executor in crates/rlm-core/src/backend_router.rs

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should work independently - multiple backends with streaming support

---

## Phase 6: User Story 4 - Performance Monitoring and Observability (Priority: P2)

**Goal**: DevOps teams can monitor RLM server performance, token usage, recursive patterns, and error rates through structured logging and metrics

**Independent Test**: Make requests while monitoring /health and /metrics endpoints to verify comprehensive observability data capture

### Integration Tests for User Story 4

- [x] T054 [P] [US4] Create metrics endpoint test in crates/rlm-server/tests/contract/test_metrics.rs
- [x] T055 [P] [US4] Create structured logging validation test in tests/integration/test_observability.rs

### Observability Implementation for User Story 4

- [x] T056 [P] [US4] Implement health check endpoint in crates/rlm-server/src/server/health.rs
- [x] T057 [P] [US4] Implement Prometheus metrics endpoint in crates/rlm-server/src/server/metrics.rs
- [x] T058 [P] [US4] Create performance metrics collection in crates/rlm-core/src/metrics.rs
- [x] T059 [US4] Add comprehensive tracing spans to RLM executor in crates/rlm-core/src/executor.rs
- [x] T060 [US4] Implement token usage tracking across recursive calls in crates/rlm-core/src/token_tracker.rs
- [x] T061 [US4] Add error rate monitoring and alerting in crates/rlm-server/src/monitoring.rs
- [x] T062 [US4] Create structured log context for recursive call chains in crates/rlm-core/src/log_context.rs
- [ ] T063 [US4] Add HTTP middleware for request/response logging in crates/rlm-server/src/server/middleware.rs

**Checkpoint**: At this point, all P1/P2 user stories are complete - full observability of RLM processing

---

## Phase 7: User Story 5 - Library Integration Support (Priority: P3)

**Goal**: Rust developers can import RLM core functionality as library crates in their own applications, independent of HTTP server

**Independent Test**: Create minimal Rust application importing rlm-core and demonstrate functionality without HTTP server dependencies

### Integration Tests for User Story 5

- [ ] T064 [P] [US5] Create library usage example in examples/library_integration.rs
- [ ] T065 [P] [US5] Create embedded usage test in examples/embedded_usage.rs

### Library Integration Implementation for User Story 5

- [ ] T066 [P] [US5] Create high-level RLM client API in crates/rlm-core/src/client.rs
- [ ] T067 [P] [US5] Implement builder pattern for RLM configuration in crates/rlm-core/src/builder.rs
- [ ] T068 [US5] Add comprehensive documentation with examples in crates/rlm-core/src/lib.rs
- [ ] T069 [US5] Create custom backend integration guide in examples/custom_backend.rs
- [ ] T070 [US5] Implement WASM-compatible bindings in crates/rlm-ffi/ (per original plan)
- [ ] T071 [US5] Add library-first usage examples in examples/basic_usage.rs
- [ ] T072 [US5] Document library API in README.md for each crate

**Checkpoint**: All user stories are now independently functional - library can be used with or without HTTP server

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Paper benchmark validation and production readiness improvements

- [ ] T073 [P] Setup complete S-NIAH golden test dataset in tests/fixtures/s_niah/
- [ ] T074 [P] Setup OOLONG aggregation dataset in tests/fixtures/oolong/
- [ ] T075 [P] Setup BROWSECOMP multi-hop QA dataset in tests/fixtures/browsecomp/
- [ ] T076 Create paper benchmark validation suite in tests/benchmarks/paper_validation.rs
- [ ] T077 [P] Implement performance benchmarking against paper baselines in tests/benchmarks/performance.rs
- [ ] T078 [P] Add comprehensive API documentation in crates/rlm-server/src/docs/
- [ ] T079 [P] Create deployment guide documentation in docs/deployment.md
- [ ] T080 Code review and cleanup across all crates for production readiness
- [ ] T081 [P] Security audit for Rhai REPL sandboxing configuration
- [ ] T082 [P] Performance optimization for large context processing
- [ ] T083 Validate quickstart.md examples against actual implementation
- [ ] T084 [P] Create Docker containerization setup in Dockerfile
- [ ] T085 [P] Setup CI/CD pipeline configuration in .github/workflows/

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - User stories can proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P1 → P2 → P2 → P3)
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Basic long-context completion - Foundation for all others
- **User Story 2 (P1)**: Streaming support - Independent of US1 but builds on same foundation
- **User Story 3 (P2)**: Multi-backend - Can start after Foundational, integrates with US1/US2
- **User Story 4 (P2)**: Observability - Independent implementation, adds monitoring to existing features
- **User Story 5 (P3)**: Library integration - Depends on US1 core functionality being stable

### Within Each User Story

- Integration tests MUST be written and FAIL before implementation
- Core entities and types before services
- Ports/traits before adapters
- Adapters before integration
- Core functionality before HTTP endpoints
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run simultaneously on different crates
- All Foundational tasks marked [P] can run in parallel within Phase 2
- Once Foundational completes, User Stories 1 & 2 can start in parallel (both P1)
- User Stories 3 & 4 can start in parallel after their prerequisites (both P2)
- All tests for a user story marked [P] can run in parallel
- Different developers can work on different user stories simultaneously

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together:
Task: "Create OpenAI API contract test for /v1/chat/completions in crates/rlm-server/tests/contract/test_openai_api.rs"
Task: "Create RLM core executor unit tests in crates/rlm-core/tests/executor_tests.rs"
Task: "Create long-context integration test in tests/integration/test_long_context.rs"

# Launch all entity models for User Story 1 together:
Task: "Implement RlmSession entity in crates/rlm-core/src/session.rs"
Task: "Implement RecursiveCallTree entity in crates/rlm-core/src/recursive_call.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 Only - Both P1)

1. Complete Phase 1: Setup → Cargo workspace ready
2. Complete Phase 2: Foundational → Core ports and types ready
3. Complete Phase 3: User Story 1 → Basic long-context completion working
4. Complete Phase 4: User Story 2 → Add streaming support
5. **STOP and VALIDATE**: Test both stories independently against paper benchmarks
6. Deploy/demo with core RLM value proposition

### Incremental Delivery

1. Complete Setup + Foundational → Core architecture ready
2. Add User Story 1 → Test independently → Basic RLM working (MVP!)
3. Add User Story 2 → Test independently → Streaming RLM complete
4. Add User Story 3 → Test independently → Multi-backend flexibility
5. Add User Story 4 → Test independently → Production monitoring ready
6. Add User Story 5 → Test independently → Library ecosystem ready
7. Each story adds incremental value without breaking previous functionality

### Parallel Team Strategy

With multiple Rust developers:

1. Team completes Setup + Foundational together → Architecture established
2. Once Foundational is done:
   - **Developer A**: User Story 1 (Core RLM) + User Story 3 (Multi-backend)
   - **Developer B**: User Story 2 (Streaming) + User Story 4 (Observability)
   - **Developer C**: User Story 5 (Library) + Phase 8 (Paper validation)
3. Stories complete and integrate through well-defined ports/adapters

---

## Notes

- **[P] tasks**: Different files/crates, no dependencies - safe for parallel execution
- **[Story] labels**: Map each task to specific user story for independent delivery
- **Paper validation**: Integration tests must prove MIT paper performance claims
- **Memory safety**: All code must compile with `#![forbid(unsafe_code)]`
- **Hexagonal architecture**: Core domain isolated from HTTP/REPL/LLM adapters
- **Production ready**: Full observability, error handling, configuration flexibility
- **Library first**: Core functionality usable without HTTP server
- **OpenAI compatible**: Drop-in replacement with transparent RLM enhancement

### Critical Success Factors

1. **Foundation First**: Phase 2 must be rock-solid before user story work
2. **Independent Testing**: Each story must work standalone per acceptance criteria
3. **Paper Benchmarks**: Golden fixtures must validate against MIT paper claims
4. **Streaming Quality**: SSE implementation must handle long-running operations gracefully
5. **Backend Flexibility**: Architecture must support any OpenAI-compatible backend
6. **Production Observability**: Full tracing/metrics for complex recursive processing