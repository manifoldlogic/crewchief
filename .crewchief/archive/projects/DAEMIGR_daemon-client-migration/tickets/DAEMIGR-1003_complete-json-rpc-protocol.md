# Ticket: DAEMIGR-1003: Complete JSON-RPC Protocol Implementation

## Status
- [x] **Task completed** - acceptance criteria met (implementation already 100% complete per DAEMIGR-1000 review)
- [x] **Tests pass** - N/A (unit tests deferred to DAEMIGR-1904 per project plan)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Complete and validate JSON-RPC 2.0 protocol handling and error serialization based on architecture specifications and DAEMIGR-1000 review findings.

## Background
The RPC and error modules (`rpc.ts`, `errors.ts`) are partially implemented. This ticket completes protocol validation, error code mapping, and error serialization to ensure correct JSON-RPC 2.0 compliance and proper DaemonError → JSON-RPC error transformation.

The daemon-client package uses line-delimited JSON-RPC 2.0 over stdin/stdout to communicate with the maproom daemon process. Proper protocol handling is critical for reliable request/response matching, error propagation, and debugging.

**Planning reference:** `.crewchief/projects/DAEMIGR_daemon-client-migration/planning/architecture.md`
- Error Serialization Format (lines 753-815)
- JSON-RPC Protocol Handling (lines 228-293)

## Acceptance Criteria
- [ ] `createRequest()` generates valid JSON-RPC 2.0 requests with:
  - `jsonrpc: "2.0"` field present
  - `method` string present
  - `params` object/array optional
  - `id` number present (from sequential counter)
- [ ] `parseResponse()` validates and parses responses:
  - Validates `jsonrpc: "2.0"` field
  - Validates `id` field present
  - Validates either `result` or `error` present (not both)
  - Throws RpcError with code -32700 for malformed JSON
  - Throws RpcError with code -32600 for invalid request structure
- [ ] Error codes mapped correctly per architecture.md specification:
  - DaemonStartError → -32000 "Daemon failed to start"
  - DaemonCrashError → -32000 "Daemon crashed"
  - DaemonTimeoutError → -32000 "Request timeout"
  - RpcTimeoutError → -32000 "RPC timeout"
  - Standard codes: -32700 (parse), -32600 (invalid request), -32601 (method not found), -32602 (invalid params), -32603 (internal error)
- [ ] Error serialization includes proper data context:
  - DaemonStartError includes cause
  - DaemonCrashError includes exitCode and signal
  - DaemonTimeoutError includes timeoutMs and method
  - RpcError preserves daemon-provided data
- [ ] Orphaned responses (no matching request ID) handled gracefully (logged, not crashed)
- [ ] Edge cases handled:
  - Empty response line (skip, log warning)
  - Response with both result and error (reject as invalid)
  - Response with null id (notification, not supported)

## Technical Requirements

### rpc.ts - RpcProtocol
- `createRequest(method: string, params: any, id: number): JsonRpcRequest`
  - Returns JsonRpcRequest object with all required fields
  - Enforces strict JSON-RPC 2.0 structure
- `parseResponse(line: string): JsonRpcResponse`
  - Validates JSON structure (throws -32700 ParseError on invalid JSON)
  - Validates JSON-RPC 2.0 fields (throws -32600 InvalidRequest on missing fields)
  - Returns validated JsonRpcResponse object
- `isError(response: JsonRpcResponse): boolean`
  - Returns true if response.error exists
  - Returns false if response.result exists
- `createError(code: number, message: string, data?: any): JsonRpcError`
  - Creates properly formatted JsonRpcError object
- Line-delimited JSON format (one JSON object per line, newline-terminated)

### errors.ts - Error Hierarchy
- **Base DaemonError class:**
  - Properties: code (string), message (string), cause (Error | undefined)
  - Extends Error with proper stack traces
- **DaemonStartError extends DaemonError:**
  - Code: 'DAEMON_START_ERROR'
  - Serializes to JSON-RPC code -32000 with cause in data
- **DaemonCrashError extends DaemonError:**
  - Code: 'DAEMON_CRASH_ERROR'
  - Properties: exitCode (number | null), signal (string | null)
  - Serializes to JSON-RPC code -32000 with exitCode and signal in data
- **DaemonTimeoutError extends DaemonError:**
  - Code: 'DAEMON_TIMEOUT_ERROR'
  - Properties: timeoutMs (number), method (string)
  - Serializes to JSON-RPC code -32000 with timeoutMs and method in data
- **RpcError extends DaemonError:**
  - Code: 'RPC_ERROR'
  - Properties: rpcCode (number), data (any)
  - Preserves original JSON-RPC error code and data
- **RpcTimeoutError extends RpcError:**
  - Code: 'RPC_TIMEOUT_ERROR'
  - Properties: rpcCode (-32000), timeoutMs (number), method (string)

### Error Serialization
- Document mapping table in errors.ts or dedicated README section
- Provide helper function for DaemonError → JsonRpcError serialization
- Include comprehensive error context in data field
- Follow standard JSON-RPC 2.0 error object format

## Implementation Notes

### Review DAEMIGR-1000 Findings
1. Check implementation-review.md or DAEMIGR-1000 ticket findings for specific gaps
2. Address any identified issues with RPC protocol validation
3. Fix any error handling incompleteness noted in review

### JSON-RPC 2.0 Compliance
Reference: https://www.jsonrpc.org/specification

**Key requirements:**
- Request must have: jsonrpc, method, id
- Request may have: params (object or array)
- Response must have: jsonrpc, id
- Response must have EITHER result OR error (never both, never neither)
- Error must have: code (number), message (string)
- Error may have: data (any)

### Error Code Strategy
- Use standard JSON-RPC codes for protocol errors (-327xx, -326xx)
- Use -32000 (ServerError) for all application-defined errors
- Distinguish error types via message and data fields
- Preserve daemon-provided error codes in RpcError.rpcCode

### Error Context Best Practices
- Include request ID in all error messages for debugging
- Include method name in timeout errors for correlation
- Include cause chain for error propagation debugging
- Provide actionable data (exitCode, signal) for error recovery

### Edge Case Handling
1. **Malformed JSON:**
   - Try/catch around JSON.parse()
   - Throw RpcError with code -32700 ParseError
   - Include original line in error data for debugging
2. **Missing fields:**
   - Validate all required fields exist
   - Throw RpcError with code -32600 InvalidRequest
   - Specify which field is missing in message
3. **Both result and error:**
   - Reject as invalid JSON-RPC response
   - Throw RpcError with code -32600
4. **Orphaned response:**
   - Log warning with response ID and content
   - Do NOT crash or throw error
   - Consider metrics/telemetry for monitoring
5. **Empty line:**
   - Skip silently or log debug message
   - Continue processing

## Dependencies
- DAEMIGR-1000 (review existing implementation - completed)
- DAEMIGR-1001 (package configuration and build setup - should be complete)

## Risk Assessment
- **Risk:** JSON parsing errors breaking daemon communication
  - **Mitigation:** Try/catch with clear error messages; include original line in error data; maintain connection even after parse errors
- **Risk:** Error code conflicts between daemon and client
  - **Mitigation:** Follow standard JSON-RPC codes; document mapping clearly; use -32000 for all application errors
- **Risk:** Missing error context making debugging hard
  - **Mitigation:** Include method, params summary, request ID in all error data; preserve full error chain with causes
- **Risk:** Protocol validation being too strict and rejecting valid responses
  - **Mitigation:** Follow JSON-RPC 2.0 spec exactly; test with various response formats; allow optional fields

## Files/Packages Affected

### Files to Modify
- `/workspace/packages/daemon-client/src/rpc.ts` - Complete protocol validation, add edge case handling
- `/workspace/packages/daemon-client/src/errors.ts` - Complete error hierarchy, add serialization helpers

### Files to Create (if needed)
- `/workspace/packages/daemon-client/src/rpc.test.ts` - Unit tests for protocol validation
- `/workspace/packages/daemon-client/src/errors.test.ts` - Unit tests for error hierarchy and serialization

### Reference Files
- `/workspace/.crewchief/projects/DAEMIGR_daemon-client-migration/planning/architecture.md` (lines 228-293, 753-815)
- JSON-RPC 2.0 Specification: https://www.jsonrpc.org/specification
