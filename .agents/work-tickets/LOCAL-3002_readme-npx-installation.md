# Ticket: LOCAL-3002: Write README with npx installation instructions

## Status
- [x] **Task completed** - README updated with npx instructions
- [x] **Tests pass** - documentation matches implementation
- [ ] **Verified** - by the verify-ticket agent

## Agents
- technical-researcher
- verify-ticket
- commit-ticket

## Summary
Create comprehensive README.md documentation for the @crewchief/maproom-mcp npm package that explains the zero-configuration setup, system requirements, and quick start guide. The README must emphasize simplicity and set clear expectations for first-time users.

## Background
The LOCAL project aims to provide a zero-configuration Maproom MCP service via npx. The success of this project depends heavily on clear, user-friendly documentation that guides users through the one-line setup process. This README serves as the primary onboarding document and must clearly communicate the value proposition: no API keys, no configuration files, just one line in .mcp.json.

The documentation must address the needs of users who may be unfamiliar with Docker or containerization, setting clear expectations about first-run behavior (model downloads, startup time) vs subsequent runs (fast startup from cache).

## Acceptance Criteria
- [ ] README.md file exists at `/workspace/packages/maproom-mcp/README.md`
- [ ] Quick start section is under 10 lines and crystal clear
- [ ] System requirements explicitly listed (Docker Desktop, RAM, disk, OS)
- [ ] First-time run expectations clearly documented (2-3 min download time, 200MB model)
- [ ] Subsequent run expectations documented (10-20 seconds startup)
- [ ] Troubleshooting section covers top 5 issues:
  - Docker Compose v2 not found
  - Port conflicts
  - Slow startup (model downloading)
  - Permission errors
  - Volume/data persistence questions
- [ ] Examples use exact .mcp.json format with proper JSON structure
- [ ] No technical jargon without clear explanation
- [ ] Links to additional resources (MCP docs, Docker installation guides)
- [ ] Architecture overview diagram or description included

## Technical Requirements

### File Location
- Path: `/workspace/packages/maproom-mcp/README.md`
- Format: Markdown with proper heading hierarchy
- Tone: Friendly, approachable, clear

### Required Sections

1. **Overview** (100-150 words)
   - What is Maproom MCP
   - Key features: zero-config, local LLM (Ollama + nomic-embed-text), offline operation
   - Benefits: no API keys, privacy-first, cost-free, works offline

2. **Quick Start** (PRIMARY FOCUS - must be < 10 lines)
   ```json
   {
     "mcpServers": {
       "maproom": {
         "command": "npx",
         "args": ["-y", "@crewchief/maproom-mcp"]
       }
     }
   }
   ```
   - Single configuration block
   - Clear statement: "That's it! No other configuration needed."

3. **System Requirements**
   - Docker Desktop (includes Compose v2 plugin)
   - 4-8GB RAM available
   - 5GB disk space
   - Supported OS: macOS, Linux, Windows with WSL2
   - Link to Docker installation: https://docs.docker.com/get-docker/

4. **What to Expect**
   - **First Run**:
     - ~2-3 minutes for Docker image download (~2GB)
     - ~200MB model download (nomic-embed-text)
     - Progress indicators will show download status
   - **Subsequent Runs**:
     - 10-20 seconds startup (containers from cache)
     - Model already downloaded

5. **Troubleshooting** (Common Issues)
   - Issue: "Docker Compose v2 not found"
     - Solution: Install Docker Desktop (not standalone docker-compose)
     - Verification: `docker compose version` should show v2.x
   - Issue: Port conflicts (11434, 5432, or MCP port in use)
     - Solution: How to change ports via environment variables
   - Issue: Slow first startup
     - Explanation: Normal - downloading model (one-time)
   - Issue: Permission errors
     - Solution: Volume permission fixes
   - Issue: Data persistence questions
     - Explanation: Where data is stored, how to backup

6. **Advanced Configuration** (Optional)
   - Custom ports via environment variables
   - Volume management and backup
   - GPU acceleration (if available)
   - Hybrid mode (local + OpenAI fallback)

7. **Architecture Overview**
   - Brief description of services (PostgreSQL, Ollama, Maproom MCP)
   - Component diagram (ASCII art or link to detailed docs)
   - How services communicate

8. **Resources & Links**
   - MCP documentation: https://modelcontextprotocol.io
   - Ollama documentation: https://ollama.ai/docs
   - pgvector documentation: https://github.com/pgvector/pgvector
   - CrewChief repository: https://github.com/johnlindquist/crewchief

### Quality Standards
- Write at 8th-grade reading level
- Use active voice
- Avoid jargon or explain when necessary
- Use code blocks with proper syntax highlighting
- Include real examples (not placeholders)
- Test all commands for accuracy
- Verify all links are valid

## Implementation Notes

### Writing Approach
1. Start with the Quick Start - this is what users see first
2. Use real user testing feedback if available
3. Reference similar successful README files:
   - https://github.com/matiassingers/awesome-readme
   - Popular npm package READMEs with high usability
4. Keep paragraphs short (2-4 sentences)
5. Use bullet points for easy scanning
6. Add emojis sparingly (only for visual hierarchy: ✅ ❌ ⚠️)

### Architecture Diagram
Consider ASCII art for simplicity:
```
┌─────────────────┐
│   IDE (Claude)  │
└────────┬────────┘
         │ stdio
         │
    ┌────▼─────────────────┐
    │  npx maproom-mcp     │  (CLI Wrapper)
    │  docker compose up   │
    └────┬─────────────────┘
         │
    ┌────▼────────────────────────┐
    │  Docker Compose Services    │
    ├─────────────────────────────┤
    │  1. Maproom MCP (Rust)      │
    │  2. PostgreSQL + pgvector   │
    │  3. Ollama + nomic-embed    │
    └─────────────────────────────┘
```

### Key Messages to Reinforce
- "One line of JSON is all you need"
- "No API keys, ever"
- "Works completely offline after first run"
- "Your code never leaves your machine"

### Testing the README
Before marking complete:
- Have a non-technical user read it
- Time how long it takes to understand the setup
- Verify all commands work as documented
- Check all links are valid
- Ensure code blocks render correctly in GitHub

## Dependencies
- **Prerequisite**: LOCAL-1008 (CLI wrapper completed, behavior documented)
  - Need to verify exact npx command behavior
  - Confirm environment variable names for advanced config
  - Validate port numbers and service names
- **External**: Access to LOCAL_ANALYSIS.md and LOCAL_ARCHITECTURE.md for technical details

## Risk Assessment
- **Risk**: README is too technical and scares away non-Docker users
  - **Mitigation**: Focus on simplicity, test with non-technical users, provide encouraging tone

- **Risk**: First-run experience is slower than documented, causing user frustration
  - **Mitigation**: Set conservative expectations (2-3 min), explain why it's one-time

- **Risk**: Troubleshooting section misses common issues
  - **Mitigation**: Monitor early user feedback, iterate quickly on documentation

- **Risk**: System requirements not clearly stated, causing installation failures
  - **Mitigation**: Bold, upfront requirements section with verification commands

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/README.md` (created)
- Potentially `/workspace/packages/maproom-mcp/package.json` (verify description/homepage fields match README)
