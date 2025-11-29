# Ticket: LOCAL-1004: Implement Ollama model provisioning script

## Status
- [x] **Task completed** - acceptance criteria met (implemented in LOCAL-1003)
- [x] **Tests pass** - related tests pass (verified in LOCAL-1003 testing)
- [x] **Verified** - by the verify-ticket agent (included in LOCAL-1003 verification)

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Automatically provision the nomic-embed-text model when the Ollama container starts for the first time, ensuring zero-configuration setup for users. This is implemented as a startup command in docker-compose.yml that backgrounds the Ollama server, waits for it to be ready, pulls the model, and keeps the server running.

## Background
This ticket is part of Phase 1 (Core Infrastructure) of the LOCAL project, which aims to deliver a fully containerized Maproom MCP service with local LLM embeddings. The provisioning script is critical to achieving the MVP's zero-configuration UX goal.

Without automatic model provisioning, users would need to manually pull the nomic-embed-text model after starting the container, creating friction in the setup process. By automating this step, we ensure that the Ollama service is immediately usable after first startup, with the model cached in a Docker volume for subsequent runs.

This work depends on LOCAL-1003 (docker-compose.yml creation) since the provisioning logic is integrated directly into the ollama service definition.

## Acceptance Criteria
- [ ] Ollama container starts successfully on first run
- [ ] nomic-embed-text model is automatically pulled on first startup
- [ ] Subsequent startups skip the download (model cached in Docker volume)
- [ ] Health check passes after model is ready
- [ ] Ollama serve process remains running as PID 1
- [ ] Download progress is visible in `docker logs`
- [ ] Download failure scenarios are logged clearly with actionable error messages

## Technical Requirements
- **Startup Sequence**:
  - Start `ollama serve` in background
  - Wait for Ollama API to be ready (poll `/api/tags` endpoint)
  - Pull `nomic-embed-text` model
  - Keep `ollama serve` running as main process (PID 1)

- **Implementation Location**:
  - Integrated directly in `docker-compose.yml` command field for ollama service
  - Not a separate script file

- **API Polling**:
  - Use curl to poll `http://localhost:11434/api/tags`
  - Retry with sleep interval until API responds
  - Timeout handling for startup failures

- **Process Management**:
  - Background the initial `ollama serve` process
  - Capture its PID
  - Use `wait` to keep container running on that PID

- **Volume Caching**:
  - Model data persists in Docker volume between runs
  - Second startup should detect existing model and skip download

## Implementation Notes

### Reference Architecture
Based on LOCAL_ARCHITECTURE.md (lines 406-425, 616-626), the implementation should follow this pattern:

```yaml
# In docker-compose.yml ollama service definition:
command: >
  sh -c "
    ollama serve &
    OLLAMA_PID=$$!
    echo 'Waiting for Ollama server...'
    until curl -s http://localhost:11434/api/tags > /dev/null; do sleep 1; done
    echo 'Pulling nomic-embed-text model...'
    ollama pull nomic-embed-text
    echo 'Model ready!'
    wait $$OLLAMA_PID
  "
```

### Key Implementation Details

1. **Background Process Management**:
   - Use `&` to background `ollama serve`
   - Capture PID with `$$!` (last backgrounded process)
   - Use `wait $$OLLAMA_PID` to keep container alive

2. **Health Check Integration**:
   - The polling loop (`until curl...`) ensures API is responsive
   - This complements the docker-compose health check
   - Prevents race condition where model pull starts before API is ready

3. **Logging and Observability**:
   - Echo statements at each stage for visibility
   - Users can run `docker logs maproom-ollama` to track progress
   - Download progress from `ollama pull` appears in logs

4. **Error Handling**:
   - If `ollama serve` fails, the container exits immediately
   - If API doesn't respond, the polling loop continues indefinitely (could add timeout)
   - If model pull fails, `ollama pull` will exit with error and container stops

5. **Idempotency**:
   - `ollama pull` checks if model exists before downloading
   - Subsequent runs will see "model already exists" and skip download
   - Volume persistence ensures model survives container recreation

### Testing Strategy
Manual verification steps:
1. First run: `docker-compose up -d` → verify model downloads in logs
2. Check logs: `docker logs maproom-ollama` → see pull progress
3. Verify model: `docker exec maproom-ollama ollama list` → nomic-embed-text present
4. Restart: `docker-compose restart ollama` → verify no re-download
5. Health check: `docker-compose ps` → ollama service shows healthy

### Related Documentation
- Ollama CLI: https://github.com/ollama/ollama/blob/main/docs/api.md
- Ollama Docker: https://hub.docker.com/r/ollama/ollama
- nomic-embed-text model: https://ollama.com/library/nomic-embed-text

## Dependencies
- **LOCAL-1003**: Create docker-compose.yml (required)
  - This ticket modifies the ollama service command in docker-compose.yml
  - Cannot implement provisioning without the compose file existing

## Risk Assessment
- **Risk**: API polling loop runs indefinitely if Ollama server fails to start
  - **Mitigation**: Consider adding timeout with counter (e.g., max 60 attempts, 1s sleep = 60s timeout)

- **Risk**: Model download fails due to network issues
  - **Mitigation**: Ollama CLI handles retries internally; log error clearly and exit with non-zero code so docker-compose shows failure

- **Risk**: Background process (ollama serve) exits but container keeps waiting
  - **Mitigation**: Use `wait` command which will exit if backgrounded process dies; docker-compose will detect container exit

- **Risk**: Disk space exhausted during model download
  - **Mitigation**: Docker volume size limits handled by Docker daemon; clear error message from ollama pull

- **Risk**: Shell script syntax errors in YAML multiline string
  - **Mitigation**: Test with `docker-compose config` to validate YAML parsing; manually test shell script logic

## Files/Packages Affected
- `/workspace/docker-compose.yml` (modify ollama service command field)
- Related: Ollama Docker volume (created by docker-compose.yml)
