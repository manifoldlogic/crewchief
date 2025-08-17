/**
 * Query Builder Helpers and Utilities
 * 
 * Provides type-safe query building utilities for the CrewChief web UI database operations.
 * Includes specialized builders for common patterns and performance optimization.
 */

import { QueryResult } from 'pg';
import { getDatabase } from './connection.js';

export interface PaginationOptions {
  page?: number;
  limit?: number;
  offset?: number;
}

export interface SortOptions {
  field: string;
  direction: 'ASC' | 'DESC';
}

export interface FilterCondition {
  field: string;
  operator: '=' | '!=' | '>' | '<' | '>=' | '<=' | 'LIKE' | 'ILIKE' | 'IN' | 'NOT IN' | 'IS NULL' | 'IS NOT NULL';
  value?: any;
  values?: any[]; // for IN/NOT IN operators
}

export interface QueryOptions {
  pagination?: PaginationOptions;
  sort?: SortOptions[];
  filters?: FilterCondition[];
  search?: {
    query: string;
    fields: string[];
  };
}

export class QueryBuilder {
  private selectFields: string[] = ['*'];
  private fromTable: string = '';
  private joinClauses: string[] = [];
  private whereConditions: string[] = [];
  private sortClauses: string[] = [];
  private limitValue?: number;
  private offsetValue?: number;
  private parameters: any[] = [];
  private parameterIndex: number = 1;

  constructor(table: string) {
    this.fromTable = table;
  }

  /**
   * Add SELECT fields
   */
  select(fields: string | string[]): this {
    if (typeof fields === 'string') {
      this.selectFields = [fields];
    } else {
      this.selectFields = fields;
    }
    return this;
  }

  /**
   * Add JOIN clause
   */
  join(table: string, condition: string, type: 'INNER' | 'LEFT' | 'RIGHT' | 'FULL' = 'INNER'): this {
    this.joinClauses.push(`${type} JOIN ${table} ON ${condition}`);
    return this;
  }

  /**
   * Add WHERE condition with parameter binding
   */
  where(condition: string, value?: any): this {
    if (value !== undefined) {
      this.whereConditions.push(condition.replace('?', `$${this.parameterIndex}`));
      this.parameters.push(value);
      this.parameterIndex++;
    } else {
      this.whereConditions.push(condition);
    }
    return this;
  }

  /**
   * Add WHERE IN condition
   */
  whereIn(field: string, values: any[]): this {
    if (values.length === 0) {
      this.whereConditions.push('FALSE'); // No results
      return this;
    }
    
    const placeholders = values.map(() => `$${this.parameterIndex++}`).join(', ');
    this.whereConditions.push(`${field} IN (${placeholders})`);
    this.parameters.push(...values);
    return this;
  }

  /**
   * Add WHERE LIKE condition for text search
   */
  whereLike(field: string, pattern: string, caseSensitive: boolean = false): this {
    const operator = caseSensitive ? 'LIKE' : 'ILIKE';
    this.whereConditions.push(`${field} ${operator} $${this.parameterIndex}`);
    this.parameters.push(`%${pattern}%`);
    this.parameterIndex++;
    return this;
  }

  /**
   * Add full-text search condition
   */
  whereFullText(field: string, query: string): this {
    this.whereConditions.push(`${field} @@ plainto_tsquery('english', $${this.parameterIndex})`);
    this.parameters.push(query);
    this.parameterIndex++;
    return this;
  }

  /**
   * Add date range condition
   */
  whereDateRange(field: string, startDate?: Date, endDate?: Date): this {
    if (startDate) {
      this.whereConditions.push(`${field} >= $${this.parameterIndex}`);
      this.parameters.push(startDate);
      this.parameterIndex++;
    }
    if (endDate) {
      this.whereConditions.push(`${field} <= $${this.parameterIndex}`);
      this.parameters.push(endDate);
      this.parameterIndex++;
    }
    return this;
  }

  /**
   * Add ORDER BY clause
   */
  orderBy(field: string, direction: 'ASC' | 'DESC' = 'ASC'): this {
    this.sortClauses.push(`${field} ${direction}`);
    return this;
  }

  /**
   * Add LIMIT clause
   */
  limit(count: number): this {
    this.limitValue = count;
    return this;
  }

  /**
   * Add OFFSET clause
   */
  offset(count: number): this {
    this.offsetValue = count;
    return this;
  }

  /**
   * Apply pagination
   */
  paginate(options: PaginationOptions): this {
    if (options.limit) {
      this.limit(options.limit);
    }
    
    if (options.offset) {
      this.offset(options.offset);
    } else if (options.page && options.limit) {
      this.offset((options.page - 1) * options.limit);
    }
    
    return this;
  }

  /**
   * Build the SQL query
   */
  build(): { sql: string; parameters: any[] } {
    let sql = `SELECT ${this.selectFields.join(', ')} FROM ${this.fromTable}`;
    
    if (this.joinClauses.length > 0) {
      sql += ` ${this.joinClauses.join(' ')}`;
    }
    
    if (this.whereConditions.length > 0) {
      sql += ` WHERE ${this.whereConditions.join(' AND ')}`;
    }
    
    if (this.sortClauses.length > 0) {
      sql += ` ORDER BY ${this.sortClauses.join(', ')}`;
    }
    
    if (this.limitValue) {
      sql += ` LIMIT ${this.limitValue}`;
    }
    
    if (this.offsetValue) {
      sql += ` OFFSET ${this.offsetValue}`;
    }
    
    return { sql, parameters: this.parameters };
  }

  /**
   * Execute the query
   */
  async execute<T = any>(): Promise<QueryResult<T>> {
    const { sql, parameters } = this.build();
    const db = getDatabase();
    return await db.query<T>(sql, parameters);
  }
}

/**
 * Specialized query builders for common patterns
 */

export class SessionQuery {
  static findByToken(token: string) {
    return new QueryBuilder('web_sessions')
      .where('auth_token = ?', token)
      .where('expires_at > NOW()')
      .where('is_active = true');
  }

  static findActiveSessionsForUser(userId: string) {
    return new QueryBuilder('web_sessions')
      .where('user_id = ?', userId)
      .where('expires_at > NOW()')
      .where('is_active = true')
      .orderBy('last_accessed', 'DESC');
  }

  static cleanupExpired() {
    return new QueryBuilder('web_sessions')
      .where('expires_at < NOW() OR is_active = false');
  }
}

export class SearchHistoryQuery {
  static findBySession(sessionId: string, options: QueryOptions = {}) {
    const query = new QueryBuilder('web_search_history')
      .where('session_id = ?', sessionId)
      .orderBy('searched_at', 'DESC');

    if (options.pagination) {
      query.paginate(options.pagination);
    }

    return query;
  }

  static findPopularQueries(timeframe: string = '7 days', limit: number = 10) {
    const sql = `
      SELECT 
        query,
        COUNT(*) as search_count,
        AVG(execution_time_ms) as avg_execution_time,
        AVG(result_count) as avg_result_count
      FROM web_search_history 
      WHERE searched_at > NOW() - INTERVAL '${timeframe}'
      GROUP BY query
      HAVING COUNT(*) > 1
      ORDER BY search_count DESC, avg_result_count DESC
      LIMIT $1
    `;
    return { sql, parameters: [limit] };
  }

  static findSimilarQueries(query: string, limit: number = 5) {
    return new QueryBuilder('web_search_history')
      .select(['query', 'COUNT(*) as frequency'])
      .whereLike('query', query)
      .orderBy('frequency', 'DESC')
      .limit(limit);
  }
}

export class AgentRunsQuery {
  static findRecent(options: QueryOptions = {}) {
    const query = new QueryBuilder('agent_runs')
      .select([
        'ar.*',
        'r.name as repo_name',
        'w.name as worktree_name'
      ])
      .join('maproom.repos r', 'ar.repo_id = r.id', 'LEFT')
      .join('maproom.worktrees w', 'ar.worktree_id = w.id', 'LEFT')
      .orderBy('started_at', 'DESC');

    if (options.filters) {
      options.filters.forEach(filter => {
        switch (filter.operator) {
          case '=':
            query.where(`${filter.field} = ?`, filter.value);
            break;
          case 'IN':
            if (filter.values) {
              query.whereIn(filter.field, filter.values);
            }
            break;
          // Add more operators as needed
        }
      });
    }

    if (options.pagination) {
      query.paginate(options.pagination);
    }

    return query;
  }

  static findByWorktree(worktreeId: number, limit: number = 50) {
    return new QueryBuilder('agent_runs')
      .where('worktree_id = ?', worktreeId)
      .orderBy('started_at', 'DESC')
      .limit(limit);
  }

  static findCompetitionRuns(competitionId: string) {
    return new QueryBuilder('agent_runs')
      .where('competition_id = ?', competitionId)
      .orderBy('competition_rank', 'ASC');
  }
}

export class WorktreeStatusQuery {
  static findSummary(repoId?: number) {
    const query = new QueryBuilder('worktree_status')
      .select([
        'worktree_id',
        'worktree_name',
        'current_branch',
        'state',
        'is_clean',
        'jsonb_array_length(active_agents) as active_agent_count',
        'commits_ahead',
        'commits_behind',
        '(modified_files + added_files + deleted_files + untracked_files) as total_changes',
        'last_scan_at',
        'last_accessed_at'
      ])
      .where('state != ?', 'archived')
      .orderBy('pinned', 'DESC')
      .orderBy('last_accessed_at', 'DESC');

    if (repoId) {
      query.where('repo_id = ?', repoId);
    }

    return query;
  }

  static findStale() {
    return new QueryBuilder('worktree_status')
      .where('last_scan_at < NOW() - INTERVAL \'1 hour\'')
      .where('state = ?', 'active');
  }

  static findActive() {
    return new QueryBuilder('worktree_status')
      .where('state = ?', 'active')
      .where('jsonb_array_length(active_agents) > 0')
      .orderBy('last_scan_at', 'DESC');
  }
}

/**
 * Helper functions for common database operations
 */

export async function executeRawQuery<T = any>(
  sql: string, 
  parameters: any[] = []
): Promise<QueryResult<T>> {
  const db = getDatabase();
  return await db.query<T>(sql, parameters);
}

export async function executeProcedure<T = any>(
  procedureName: string,
  parameters: any[] = []
): Promise<QueryResult<T>> {
  const placeholders = parameters.map((_, index) => `$${index + 1}`).join(', ');
  const sql = `SELECT * FROM ${procedureName}(${placeholders})`;
  return await executeRawQuery<T>(sql, parameters);
}

export function buildInsertQuery(
  table: string,
  data: Record<string, any>,
  onConflict?: string
): { sql: string; parameters: any[] } {
  const fields = Object.keys(data);
  const placeholders = fields.map((_, index) => `$${index + 1}`);
  const values = fields.map(field => data[field]);

  let sql = `INSERT INTO ${table} (${fields.join(', ')}) VALUES (${placeholders.join(', ')})`;
  
  if (onConflict) {
    sql += ` ${onConflict}`;
  }

  return { sql, parameters: values };
}

export function buildUpdateQuery(
  table: string,
  data: Record<string, any>,
  whereClause: string,
  whereParams: any[] = []
): { sql: string; parameters: any[] } {
  const fields = Object.keys(data);
  const setClauses = fields.map((field, index) => `${field} = $${index + 1}`);
  const values = fields.map(field => data[field]);

  let paramIndex = values.length + 1;
  const updatedWhereClause = whereClause.replace(/\?/g, () => `$${paramIndex++}`);

  const sql = `UPDATE ${table} SET ${setClauses.join(', ')} WHERE ${updatedWhereClause}`;
  const parameters = [...values, ...whereParams];

  return { sql, parameters };
}

/**
 * Type-safe result mapping utilities
 */

export function mapToEntity<T>(row: any, mapper: (row: any) => T): T {
  return mapper(row);
}

export function mapArrayToEntities<T>(rows: any[], mapper: (row: any) => T): T[] {
  return rows.map(row => mapper(row));
}

/**
 * Pagination utilities
 */

export interface PaginatedResult<T> {
  data: T[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    totalPages: number;
    hasNext: boolean;
    hasPrev: boolean;
  };
}

export async function executePaginatedQuery<T>(
  baseQuery: string,
  countQuery: string,
  parameters: any[],
  options: PaginationOptions
): Promise<PaginatedResult<T>> {
  const { page = 1, limit = 25 } = options;
  const offset = (page - 1) * limit;

  // Execute count query
  const countResult = await executeRawQuery<{ count: string }>(countQuery, parameters);
  const total = parseInt(countResult.rows[0]?.count || '0');

  // Execute data query with pagination
  const dataQuery = `${baseQuery} LIMIT $${parameters.length + 1} OFFSET $${parameters.length + 2}`;
  const dataResult = await executeRawQuery<T>(dataQuery, [...parameters, limit, offset]);

  const totalPages = Math.ceil(total / limit);

  return {
    data: dataResult.rows,
    pagination: {
      page,
      limit,
      total,
      totalPages,
      hasNext: page < totalPages,
      hasPrev: page > 1,
    },
  };
}