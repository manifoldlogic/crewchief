# Ticket: CIFIX-3003: Set up monitoring alerts (OPTIONAL)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (optional configuration task)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- verify-ticket
- commit-ticket

## Summary
**OPTIONAL TICKET - NOT REQUIRED FOR MVP**

Configure GitHub Actions notifications and optionally set up Slack/Discord webhooks to alert on workflow failures. This is a "nice to have" enhancement that can be deferred to future work without impacting core CI functionality.

## Background
With test workflow and Docker build stabilized through Phase 1 and 2 tickets, optional monitoring can provide:
1. Immediate alerts when workflows fail (faster response time)
2. Tracking of workflow success rates over time
3. Alerts for unusual conditions (slow builds, size changes)

**Why This is Optional:**
- GitHub already provides email notifications by default (no setup needed)
- CI is now stable - monitoring is enhancement not requirement
- Team can operate effectively without additional alerts
- Can be implemented later if needed (not time-sensitive)

**Decision Criteria for Implementation:**
- **Implement IF**: Team frequently misses GitHub email notifications
- **Implement IF**: Multiple developers need real-time alerts
- **Implement IF**: Historical metrics would inform optimization work
- **Defer IF**: Current notifications are sufficient
- **Defer IF**: Team prefers manual checks over automated alerts

**Recommendation**: **DEFER** - Mark as "Future Enhancement" and skip for MVP

## Acceptance Criteria (ALL OPTIONAL)
- [ ] GitHub Actions email notifications verified working
- [ ] (Optional) Slack/Discord webhook configured for workflow failures
- [ ] (Optional) Monitoring dashboard created for CI metrics
- [ ] (Optional) Alert thresholds configured (image size >230MB, build time >15min)
- [ ] Documentation added explaining how monitoring works OR documented as future enhancement

## Technical Requirements

**If Implementing:**
- **GitHub Actions**: Use workflow status checks
- **Webhooks**: Slack or Discord incoming webhook URLs
- **Metrics**: GitHub Actions API for historical data
- **Thresholds**: Configurable via environment variables

**If Deferring:**
- Add to future work backlog in CIFIX project documentation

## Implementation Notes

### Option 1: Defer (Recommended for MVP)

Add to `.agents/projects/CIFIX_ci-workflow-fixes/README.md` or final documentation:

```markdown
## Future Enhancements

### CI Monitoring (Deferred)
- Slack/Discord webhook integration for workflow failures
- Metrics dashboard for success rate and build times
- Automated alerts for anomalies (image size, build duration)
- Reference: CIFIX-3003 (optional monitoring setup)

**Current State**: GitHub email notifications are active and sufficient for current needs.
**Rationale**: CI is stable, additional monitoring is enhancement not requirement.
```

### Option 2: Implement Basic Monitoring

**Verify GitHub Email Notifications** (no code changes needed):
```bash
# Already configured by GitHub automatically
# Verify in GitHub Settings → Notifications → Actions
# Each user controls their notification preferences
```

**Optional Slack Integration** (if implementing):

1. Add Slack webhook secret to repository:
```bash
# In GitHub Settings → Secrets and Variables → Actions
# Add: SLACK_WEBHOOK_URL
```

2. Add notification step to workflows:
```yaml
# In .github/workflows/test.yml and publish-maproom-mcp-image.yml
- name: Notify on failure
  if: failure()
  uses: slackapi/slack-github-action@v1
  with:
    webhook-url: ${{ secrets.SLACK_WEBHOOK_URL }}
    payload: |
      {
        "text": "CI Failure: ${{ github.workflow }} failed on ${{ github.ref }}",
        "blocks": [
          {
            "type": "section",
            "text": {
              "type": "mrkdwn",
              "text": "*Workflow Failed*\n*Workflow:* ${{ github.workflow }}\n*Branch:* ${{ github.ref }}\n*Commit:* ${{ github.sha }}"
            }
          },
          {
            "type": "actions",
            "elements": [
              {
                "type": "button",
                "text": {
                  "type": "plain_text",
                  "text": "View Run"
                },
                "url": "${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}"
              }
            ]
          }
        ]
      }
```

**Optional Metrics Dashboard** (if implementing):
- Use GitHub Actions metrics API
- Track: success rate, duration, image size over time
- Visualize with GitHub Pages or external service (Grafana, etc.)

### Validation (If Implementing)

```bash
# Test webhook manually
curl -X POST $SLACK_WEBHOOK_URL \
  -H 'Content-Type: application/json' \
  -d '{"text":"Test notification from CIFIX monitoring setup"}'

# Verify GitHub notifications configured
gh api /repos/:owner/:repo/actions/workflows

# Check workflow runs history
gh run list --limit 10
```

## Dependencies
- **Requires**: CIFIX-3001 (documentation context), CIFIX-3002 (monitoring documentation context)
- **Blocks**: None

## Risk Assessment
- **Risk**: None (optional enhancement)
  - **Mitigation**: N/A - can be safely deferred or skipped

- **Risk**: Alert fatigue if thresholds too sensitive
  - **Mitigation**: Start with failure-only alerts, add metrics alerts later based on need

- **Risk**: Webhook URL leakage if not properly secured
  - **Mitigation**: Use GitHub Secrets, never commit webhook URLs to repository

## Files/Packages Affected

**If Implementing:**
- `.github/workflows/test.yml` - Add failure notification
- `.github/workflows/publish-maproom-mcp-image.yml` - Add failure notification
- `.github/CLAUDE.md` - Document monitoring setup
- Repository secrets (SLACK_WEBHOOK_URL or DISCORD_WEBHOOK_URL)

**If Deferring:**
- `.agents/projects/CIFIX_ci-workflow-fixes/README.md` - Document as future enhancement
- OR `.github/CLAUDE.md` - Add to future work section

## Estimated Time
- **If Deferred**: 2 minutes (document as future enhancement)
- **If Implemented**: 20-30 minutes (webhook setup + testing)

## Suggested Action
**SKIP** this ticket for MVP and mark as "Future Enhancement" in project documentation. GitHub email notifications are already active and sufficient for current CI stability monitoring needs.
