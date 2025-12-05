# Security Review: Make mxbai-embed-large the Default Model

## Security Assessment

**Overall Risk Level**: **NONE**

This is a configuration change that updates default values. No security-sensitive code is modified, and no new security concerns are introduced.

## Changes Analysis

### Modified Code

1. **ollama.rs** (2 constant changes):
   - `DEFAULT_MODEL = "mxbai-embed-large"` (was "nomic-embed-text")
   - `default_config()` dimension = 1024 (was 768)

2. **factory.rs** (1 fallback change):
   - Fallback model = "mxbai-embed-large" (was "nomic-embed-text")

**Security Impact**: None. These are read-only constants and local variable values.

### Modified Documentation

- Multiple documentation files updated to show new defaults
- Migration guide created

**Security Impact**: None. Documentation changes do not affect code execution.

## Security Considerations

### Authentication & Authorization

**Impact**: None

**Rationale**: No authentication or authorization code modified. Model selection happens after authentication (if any).

### Data Protection

**Impact**: None

**Rationale**:
- No changes to data encryption
- No changes to data storage (same virtual table pattern)
- No changes to data transmission
- Embeddings remain local (no cloud transmission)

**Note**: mxbai-embed-large actually **improves** data integrity by not mangling content during sanitization.

### Input Validation

**Impact**: Positive (removal of sanitization for default model)

**Rationale**:
- nomic-embed-text required character sanitization as a **workaround** for GGML bugs
- mxbai-embed-large handles all input correctly without sanitization
- Sanitization is **preserved** for nomic-embed-text users (backward compat)

**Security benefit**: Less data transformation = fewer opportunities for bugs.

### Dependency Security

**Impact**: None

**Rationale**:
- Both models come from same source (Ollama)
- No new dependencies introduced
- No dependency version changes

**Model Source**:
- nomic-embed-text: Nomic AI via Ollama
- mxbai-embed-large: MixedBread AI via Ollama

Both are open-source, well-vetted embedding models.

### Environment Variables

**Impact**: None

**Rationale**:
- Same environment variable pattern (`MAPROOM_EMBEDDING_MODEL`, `MAPROOM_EMBEDDING_DIMENSION`)
- No new environment variables added
- No credential handling changes

**Backward compatibility**: Explicit configuration still works exactly as before.

## Threat Model

### Threat: Malicious Model Download

**Scenario**: Attacker replaces mxbai-embed-large model with malicious version

**Likelihood**: Very Low

**Impact**: High (if successful)

**Mitigation**:
- Ollama verifies model checksums
- Models downloaded from official Ollama registry
- User controls Ollama installation

**Our Responsibility**: None. Model download/verification is Ollama's responsibility.

**User Action**: Users should ensure they're using legitimate Ollama installation.

### Threat: Model Behavior Difference

**Scenario**: mxbai-embed-large behaves unexpectedly, leaking information

**Likelihood**: Very Low

**Impact**: Medium

**Mitigation**:
- mxbai-embed-large is open-source and well-vetted
- Runs locally (no network transmission)
- Same local-only architecture as nomic-embed-text

**Our Responsibility**: Document model behavior, provide opt-out via configuration.

**User Action**: Users concerned about model can explicitly configure nomic-embed-text.

### Threat: Backward Compatibility Breaks Security Config

**Scenario**: Users relying on sanitization lose it unintentionally

**Likelihood**: Very Low

**Impact**: Low

**Mitigation**:
- Sanitization is not a security feature (it's a bug workaround)
- Conditional sanitization preserved for nomic-embed-text
- Explicit configuration allows users to choose

**Our Responsibility**: Document sanitization behavior clearly.

**User Action**: None. Users who care about sanitization can stay on nomic-embed-text.

## Compliance

### Data Privacy

**Impact**: Positive

**Rationale**:
- All processing remains local (no cloud transmission)
- No change to data retention
- Better data fidelity (no content mangling)

**GDPR/Privacy**: No impact. Same local-only architecture.

### Audit Trail

**Impact**: None

**Rationale**: No changes to logging or audit functionality.

**Recommendation**: Document in migration guide that model choice is logged in application logs.

## Known Security Gaps

### Gap 1: Model Authenticity

**Description**: No built-in verification that downloaded model is authentic

**Severity**: Low

**Rationale**: Ollama handles this, not our code

**Mitigation**: Document that users should use official Ollama installation

**Acceptance**: This is Ollama's responsibility, not ours. Gap accepted.

### Gap 2: Model Behavior Audit

**Description**: No automated testing of model output for security issues

**Severity**: Very Low

**Rationale**: Embedding models are deterministic and well-vetted

**Mitigation**: Use reputable, open-source models (mxbai-embed-large qualifies)

**Acceptance**: Gap accepted. Manual model selection addresses this.

## Risk Acceptance

**All security risks associated with this change are accepted because**:

1. **Configuration change only**: No new attack surface
2. **Backward compatible**: Users can revert to old model
3. **Local-only processing**: No network transmission changes
4. **Reputable models**: Both nomic-embed-text and mxbai-embed-large are well-vetted
5. **Open source**: Both models are open-source and auditable
6. **User control**: Users can explicitly choose model via configuration

## Recommendations

### For Users

1. **Verify Ollama installation**: Ensure you're using official Ollama from ollama.ai
2. **Review model choice**: If concerned about specific model behavior, explicitly configure nomic-embed-text
3. **Monitor logs**: Application logs will show which model is being used

### For Documentation

1. **Document model sources**: Mention that models come from Ollama registry
2. **Document opt-out**: Show how to stay on nomic-embed-text if desired
3. **Document behavior difference**: Explain sanitization vs no-sanitization

### For Future Development

1. **Model provenance**: Consider adding model checksum verification (if Ollama doesn't provide)
2. **Configuration validation**: Consider warning users if model/dimension mismatch
3. **Telemetry**: Consider collecting anonymous stats on model usage (opt-in)

## Security Testing

### Required Tests

**None**. This is a configuration change with no security implications.

### Recommended Tests

1. **Manual verification**: Confirm mxbai-embed-large processes sensitive content locally (no network)
2. **Log inspection**: Verify model choice is logged correctly
3. **Configuration override**: Verify explicit model config works (prevents accidental exposure)

### Passed Security Checks

- **Static analysis**: No new code, only constants changed
- **Dependency scan**: No new dependencies
- **Secret scanning**: No secrets in code or docs
- **Code review**: Configuration changes reviewed

## Conclusion

**This change introduces NO meaningful security concerns.**

The configuration change from nomic-embed-text to mxbai-embed-large:
- Does not modify security-sensitive code
- Does not introduce new attack vectors
- Maintains backward compatibility
- Preserves local-only processing
- Uses reputable, open-source model
- Allows user override via configuration

**Recommendation**: Proceed with change. No security blockers identified.

## Sign-Off

**Security Assessment**: APPROVED

**Risk Level**: NONE

**Blockers**: None

**Conditions**: None

**Next Steps**: Proceed with implementation per execution plan.
