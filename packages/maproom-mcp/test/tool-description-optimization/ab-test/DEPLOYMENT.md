# A/B Testing Infrastructure Deployment Guide

This guide covers deploying the A/B testing infrastructure for production variant testing.

## Overview

The A/B testing infrastructure enables live traffic comparison of tool description variants:

- **Variant Assignment**: Consistent hashing assigns users to variants
- **Metrics Collection**: Logs query performance with variant information
- **Statistical Analysis**: Detects winners with statistical significance
- **Dashboard**: Monitors experiment progress

## Prerequisites

- PostgreSQL database with pgvector extension
- Node.js 18+ runtime
- Access to MCP server codebase

## Database Setup

### 1. Create Metrics Table

Run this SQL against your PostgreSQL database:

```sql
CREATE TABLE IF NOT EXISTS ab_test_metrics (
  id SERIAL PRIMARY KEY,
  timestamp BIGINT NOT NULL,
  user_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  variant TEXT NOT NULL,
  query_original TEXT NOT NULL,
  result_count INT NOT NULL,
  success BOOLEAN NOT NULL,
  execution_time_ms INT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ab_test_variant ON ab_test_metrics(variant);
CREATE INDEX IF NOT EXISTS idx_ab_test_timestamp ON ab_test_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_ab_test_user ON ab_test_metrics(user_id);
```

### 2. Verify Table Creation

```bash
psql -U maproom -d maproom -c "\d ab_test_metrics"
```

## Integration Steps

### 1. Modify MCP Server

Add variant assignment to `packages/maproom-mcp/src/index.ts`:

```typescript
import { assignVariant, create90_10Split } from './test/tool-description-optimization/ab-test/assigner.js'
import { ABTestCollector } from './test/tool-description-optimization/ab-test/collector.js'

// Initialize collector
const abTestCollector = new ABTestCollector({
  successThreshold: 3,
  storage: 'postgres' // or 'memory' for testing
})

// In tool description handler
server.setRequestHandler(ListToolsRequestSchema, async (request) => {
  // Get user identifier (from MCP context or generate)
  const userId = request.params?.userId || generateSessionId()

  // Assign variant (start with 90/10 split for safety)
  const splits = create90_10Split('baseline', 'improved')
  const assignment = assignVariant(userId, splits)

  // Load tool description for assigned variant
  const toolDescription = loadVariantDescription(assignment.variant_id)

  // Return tools with assigned variant description
  return {
    tools: [
      {
        name: 'search',
        description: toolDescription,
        inputSchema: { /* ... */ }
      }
    ]
  }
})

// In search tool handler
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  // Execute search
  const results = await executeSearch(request.params.arguments.query)

  // Log metrics
  abTestCollector.log({
    user_id: userId,
    session_id: sessionId,
    variant: assignment.variant_id,
    query_original: request.params.arguments.query,
    result_count: results.length,
    execution_time_ms: executionTime
  })

  return { content: results }
})
```

### 2. Configure Traffic Split

Start conservative with 90/10 split:

```typescript
// 90% baseline (current production), 10% experiment
const splits = create90_10Split('variant-control', 'variant-improved')
```

After confidence builds, move to 50/50:

```typescript
// 50/50 split for faster statistical significance
const splits = create50_50Split('variant-control', 'variant-improved')
```

### 3. Deploy Monitoring

#### Option A: Simple Dashboard (Recommended for MVP)

```typescript
import { displayDashboard } from './test/tool-description-optimization/ab-test/dashboard.js'

// Display dashboard in terminal
setInterval(() => {
  console.clear()
  displayDashboard(abTestCollector, 'experiment-1')
}, 60000) // Update every minute
```

#### Option B: HTTP Endpoint

```typescript
import { createDashboardHandler } from './test/tool-description-optimization/ab-test/dashboard.js'

app.get('/ab-test/dashboard', createDashboardHandler(abTestCollector))
```

Access dashboard:
```bash
curl http://localhost:3000/ab-test/dashboard
curl http://localhost:3000/ab-test/dashboard?format=text
```

## Monitoring and Analysis

### Real-time Monitoring

Watch dashboard continuously:

```bash
watch -n 60 "curl -s http://localhost:3000/ab-test/dashboard?format=text"
```

### Statistical Analysis

Run analysis when sample size ≥1000 per variant:

```typescript
import { analyzeExperiment } from '../analyzer.js'

// Convert collector metrics to VariantMetrics format
const variantMetrics = convertToVariantMetrics(abTestCollector)

// Run statistical analysis
const analysis = analyzeExperiment(variantMetrics, 'production-exp-1')

console.log(generateReport(analysis))
```

### SQL Queries

Direct database analysis:

```sql
-- Success rate by variant
SELECT
  variant,
  COUNT(*) as total,
  SUM(CASE WHEN success THEN 1 ELSE 0 END) as successful,
  AVG(CASE WHEN success THEN 1.0 ELSE 0.0 END) as success_rate
FROM ab_test_metrics
WHERE timestamp >= EXTRACT(EPOCH FROM NOW() - INTERVAL '24 hours') * 1000
GROUP BY variant;

-- Top failing queries by variant
SELECT
  variant,
  query_original,
  COUNT(*) as failure_count
FROM ab_test_metrics
WHERE success = false
GROUP BY variant, query_original
ORDER BY failure_count DESC
LIMIT 20;
```

## Safety and Rollback

### Instant Rollback

If metrics show degradation:

```typescript
// Revert to 100% baseline
const splits = [
  { variant_id: 'baseline', variant_name: 'baseline', percentage: 100 }
]
```

### Gradual Ramp-up

If winner detected:

```typescript
// Week 1: 90/10 split
// Week 2: 75/25 split (if stable)
// Week 3: 50/50 split (for faster data)
// Week 4: 100% winner (if p<0.01)
```

### Automated Safety Checks

```typescript
setInterval(() => {
  const summary = abTestCollector.getSummary()

  for (const [variant, stats] of summary.entries()) {
    // Alert if success rate drops below 60%
    if (stats.success_rate < 0.6) {
      console.error(`⚠️  ALERT: ${variant} success rate dropped to ${stats.success_rate}`)
      // Trigger rollback
    }
  }
}, 300000) // Check every 5 minutes
```

## Winner Detection Criteria

A variant is declared winner when:

1. **Statistical Significance**: p-value < 0.05 (with Bonferroni correction)
2. **Practical Significance**: Improvement > 5% success rate
3. **Sample Size**: n ≥ 1000 per variant
4. **Stability**: Consistent performance over 7 days

## Post-Deployment

### 1. Deploy Winner

Update production tool description:

```typescript
// packages/maproom-mcp/src/index.ts
const PRODUCTION_DESCRIPTION = loadVariantDescription('variant-improved')
```

### 2. Archive Experiment

```sql
-- Archive metrics for future reference
CREATE TABLE ab_test_archive_20250106 AS
SELECT * FROM ab_test_metrics
WHERE timestamp >= ... AND timestamp <= ...;

-- Clean up active table
DELETE FROM ab_test_metrics WHERE timestamp < ...;
```

### 3. Plan Next Iteration

Use winner as new baseline for next experiment:

```typescript
const splits = create90_10Split('variant-improved', 'variant-next-gen')
```

## Troubleshooting

### Issue: Inconsistent Assignment

**Symptom**: Same user gets different variants

**Fix**: Verify hash function stability:

```typescript
import { testAssignmentStability } from './assigner.js'

const stable = testAssignmentStability('test-user-123', splits, 1000)
console.log('Assignment stable:', stable) // Should be true
```

### Issue: Low Sample Size

**Symptom**: Not reaching 1000 samples after 1 week

**Fix**: Increase traffic allocation or extend duration

### Issue: Database Connection Errors

**Symptom**: Metrics not being logged

**Fix**: Check PostgreSQL connection and table permissions

```sql
GRANT INSERT, SELECT ON ab_test_metrics TO maproom;
```

## Best Practices

1. **Start Small**: Begin with 10% traffic to experiment
2. **Monitor Closely**: Check dashboard hourly in first 24 hours
3. **Set Alerts**: Automated alerts for success rate drops
4. **Document Everything**: Log all configuration changes
5. **Regular Analysis**: Run statistical analysis weekly
6. **Gradual Rollout**: Increase traffic slowly as confidence builds

## Support

For questions or issues:
- Check logs: `tail -f /var/log/maproom-mcp.log`
- Database metrics: Query `ab_test_metrics` table directly
- Code: `packages/maproom-mcp/test/tool-description-optimization/ab-test/`
