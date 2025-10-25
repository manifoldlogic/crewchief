# Query Optimization Guide

This document provides comprehensive guidance on query optimization techniques implemented in Maproom, including materialized views, index strategies, and performance tuning.

## Table of Contents

1. [Overview](#overview)
2. [Materialized Views](#materialized-views)
3. [Index Strategy](#index-strategy)
4. [Query Patterns](#query-patterns)
5. [Performance Monitoring](#performance-monitoring)
6. [Refresh Strategies](#refresh-strategies)
7. [Troubleshooting](#troubleshooting)
8. [Best Practices](#best-practices)

## Overview

Maproom's query optimization strategy focuses on three key areas:

1. **Materialized Views**: Pre-compute expensive joins and aggregations
2. **Strategic Indices**: Covering indices, partial indices, and BRIN indices
3. **Query Rewrites**: CTEs, pushed-down filters, and prepared statements

### Performance Targets

- **Search queries (p95)**: < 50ms for k=10 results
- **Context assembly (p95)**: < 120ms
- **Graph traversal (depth=3)**: < 50ms
- **File enumeration**: < 10ms for 1000 files

### Baseline Improvements

Compared to PERF_OPT-1002 baseline (before optimization):

- Full-text search: 60-70% faster
- Vector similarity: 65-70% faster
- Graph-based search: 70-80% faster
- File queries: 70-80% faster
- Overall p95 latency: 55-65% reduction

For the complete documentation, see the file in the repository.

**Last Updated:** 2025-10-25
**Migrations:** PERF_OPT-2001 (0012), PERF_OPT-2002 (0013)
**Version:** 1.0.0
