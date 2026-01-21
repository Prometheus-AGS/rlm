/**
 * RLM WASM FFI TypeScript Definitions
 * Generated for Cherry Studio integration
 */

/**
 * Execute an RLM request through WASM.
 * @param request_json - JSON-encoded RlmRequest
 * @param event_callback - Callback function for streaming events
 * @returns Promise resolving to RlmResponse JSON
 */
export function rlm_execute(
  request_json: RlmRequest,
  event_callback: (event: RlmEvent) => void
): Promise<RlmResponse>;

/**
 * Request to execute RLM.
 */
export interface RlmRequest {
  /** User query to answer */
  query: string;
  /** Long context to offload to REPL */
  context: string;
  /** Maximum REPL iterations (default: 50) */
  max_iterations?: number;
  /** Maximum recursive call depth (default: 1) */
  recursion_depth?: number;
  /** Optional metadata for tracking */
  metadata?: Record<string, string>;
}

/**
 * Response from RLM execution.
 */
export interface RlmResponse {
  /** Final answer to the query */
  answer: string;
  /** Execution metadata */
  metadata: ExecutionMetadata;
  /** REPL final state (for debugging) */
  repl_state?: string;
}

/**
 * Metadata about execution.
 */
export interface ExecutionMetadata {
  /** Number of REPL iterations used */
  iterations: number;
  /** Number of recursive LLM calls made */
  recursive_calls: number;
  /** Total tokens consumed */
  total_tokens: number;
  /** Execution duration in milliseconds */
  duration: number;
  /** Start timestamp (unix seconds) */
  started_at: number;
  /** Whether execution completed successfully */
  success: boolean;
}

/**
 * RLM streaming event types.
 */
export type RlmEvent =
  | { type: 'context_chunk'; chunk_id: string; size_bytes: number; processed: boolean }
  | { type: 'repl_op'; iteration: number; code: string; result: ReplResult }
  | { type: 'recursive_call'; call_id: string; depth: number; status: CallStatus; prompt_tokens: number; completion_tokens: number }
  | { type: 'chunk'; content: string; chunk_index: number; is_final: boolean }
  | { type: 'done'; answer: string; metadata: ExecutionMetadata }
  | { type: 'error'; error_type: string; message: string; recoverable: boolean };

/**
 * REPL operation result.
 */
export type ReplResult =
  | { status: 'success'; value: string }
  | { status: 'error'; message: string };

/**
 * Recursive call status.
 */
export type CallStatus = 'pending' | 'in_progress' | 'completed' | 'failed';
