# Ticket: CFGVER-6003: Post-Release Monitoring

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- documentation-engineer
- verify-ticket
- commit-ticket

## Summary
Monitor for issues and collect user feedback after release. This includes watching GitHub issues, npm download stats, and user discussions for 48 hours post-release. Document any issues discovered and prepare hotfix if critical issues found.

## Background
After release, monitor for problems or feedback from users actually using the new version management system. Early detection of issues allows quick response with hotfixes. Positive feedback validates the release. User feedback informs future enhancements.

Reference: `plan.md` lines 143-164 for Phase 6 monitoring objectives.

## Acceptance Criteria
- [ ] Monitor GitHub issues for 48 hours post-release
- [ ] Check npm download stats daily
- [ ] Respond to user questions within 24 hours
- [ ] Document any critical issues discovered
- [ ] Prepare hotfix if critical issues found
- [ ] Create post-release report with findings
- [ ] Update FAQ based on user questions

## Technical Requirements

**Monitoring Activities:**

**Day 1-2 After Release:**
- Check GitHub issues every 4 hours
- Monitor npm downloads: `npm view @crewchief/maproom-mcp --json | jq .downloads`
- Watch for error patterns in issues
- Respond to questions/problems promptly
- Document recurring issues

**Success Metrics:**
- Zero critical issues reported
- < 5 minor issues reported
- Positive user feedback in issues/discussions
- No rollback needed
- npm downloads increasing

**Issue Categories:**

1. **Critical (P0)** - Config updates failing, data loss
   - **Response:** Immediate hotfix within 4 hours
   - **Example:** Rollback fails, user configs deleted

2. **High (P1)** - Updates work but error messages unclear
   - **Response:** Document workaround, plan fix in 1.2.4
   - **Example:** Docker errors cryptic, users confused

3. **Medium (P2)** - Edge cases not handled
   - **Response:** Document limitation, plan enhancement
   - **Example:** Concurrent updates not supported

4. **Low (P3)** - Nice-to-have features
   - **Response:** Add to backlog for future release
   - **Example:** Dry run mode for updates

**Monitoring Channels:**
- GitHub Issues: https://github.com/{org}/{repo}/issues
- GitHub Discussions: https://github.com/{org}/{repo}/discussions
- npm package page: https://www.npmjs.com/package/@crewchief/maproom-mcp
- Package download stats: `npm view @crewchief/maproom-mcp`

## Implementation Notes

**GitHub Issue Triage Process:**

1. **New Issue Created**
   - Read issue description thoroughly
   - Ask for reproduction steps if unclear
   - Categorize priority (P0-P3)
   - Respond within 24 hours

2. **Critical Issue (P0)**
   - Reproduce locally immediately
   - Create hotfix branch
   - Fix and test
   - Release 1.2.4 patch
   - Update issue with resolution

3. **High/Medium Issue (P1-P2)**
   - Document workaround if available
   - Add to TROUBLESHOOTING.md if common
   - Create follow-up ticket for fix
   - Respond with workaround and timeline

4. **Low Priority Issue (P3)**
   - Thank user for feedback
   - Add to backlog
   - Respond with acknowledgment

**npm Download Stats:**

Check daily:
```bash
# View package stats
npm view @crewchief/maproom-mcp

# Check version distribution
npm view @crewchief/maproom-mcp dist-tags

# Monitor downloads (if unpkg available)
curl https://api.npmjs.org/downloads/point/last-week/@crewchief/maproom-mcp
```

**User Response Templates:**

**Critical Issue Response:**
```markdown
Thank you for reporting this critical issue. We're investigating immediately and will release a hotfix ASAP.

In the meantime, you can rollback to the previous version:
```bash
npm install -g @crewchief/maproom-mcp@1.2.2
```

We'll update this issue as soon as we have a fix.
```

**Workaround Response:**
```markdown
Thank you for reporting this issue. We've identified a workaround:

[Workaround steps]

We'll address this properly in version 1.2.4. In the meantime, please let us know if this workaround resolves your issue.
```

**Post-Release Report:**

Create: `.crewchief/projects/CFGVER_config-version-management/post-release-report.md`

**Template:**
```markdown
# CFGVER Post-Release Report

**Release:** v1.2.3 - Config Version Management
**Published:** 2024-11-XX
**Monitoring Period:** 48 hours

## Metrics

- **npm Downloads:** [number] in first 48 hours
- **GitHub Issues:** [number] reported
- **Critical Issues (P0):** [number]
- **High Issues (P1):** [number]
- **Medium Issues (P2):** [number]
- **Low Issues (P3):** [number]

## Issues Summary

### Critical (P0)
[None or list with links]

### High (P1)
[None or list with links]

### Medium (P2)
[None or list with links]

### Low (P3)
[None or list with links]

## User Feedback

**Positive:**
- [Quote or summary]

**Negative:**
- [Quote or summary]

## Lessons Learned

**What Went Well:**
- [List successes]

**What Could Improve:**
- [List improvements for next release]

## Follow-Up Actions

- [ ] Create tickets for reported issues
- [ ] Update TROUBLESHOOTING.md with common issues
- [ ] Plan hotfix if needed (1.2.4)
- [ ] Update FAQ based on questions

## Conclusion

[Summary of release success/issues]
```

## Dependencies
- CFGVER-6002 (package published)

## Risk Assessment
- **Risk**: Critical issues discovered (users can't connect to MCP)
  - **Mitigation**: Hotfix process ready, can release 1.2.4 quickly

- **Risk**: High support burden (many user questions)
  - **Mitigation**: Prepare FAQ, update TROUBLESHOOTING.md

- **Risk**: Negative user feedback damages reputation
  - **Mitigation**: Respond quickly and professionally, provide workarounds

- **Risk**: Issues only appear on specific platforms (Windows, Linux)
  - **Mitigation**: Ask users for platform details, test on those platforms

## Files/Packages Affected
- **Create**: `.crewchief/projects/CFGVER_config-version-management/post-release-report.md`
- **Modify**: `packages/maproom-mcp/docs/TROUBLESHOOTING.md` (add common issues)
- **Modify**: `packages/maproom-mcp/CHANGELOG.md` (if hotfix needed)
