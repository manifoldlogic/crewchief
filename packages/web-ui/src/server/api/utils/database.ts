import { type Pool } from 'pg';
import { type PaginationQuery, type PaginationResponse } from '../schemas/common.js';

// Database query builder utility
export class ApiQueryBuilder {
  private tableName: string;
  private whereConditions: string[] = [];
  private whereParams: any[] = [];
  private selectFields: string[] = ['*'];
  private orderByClause = '';
  private limitClause = '';
  private offsetClause = '';
  private joinClauses: string[] = [];
  private paramIndex = 1;

  constructor(tableName: string) {
    this.tableName = tableName;
  }

  select(fields: string | string[]): this {
    if (typeof fields === 'string') {
      this.selectFields = [fields];
    } else {
      this.selectFields = fields;
    }
    return this;
  }

  where(condition: string, value?: any): this {
    if (value !== undefined) {
      this.whereConditions.push(condition.replace('?', `$${this.paramIndex++}`));
      this.whereParams.push(value);
    } else {
      this.whereConditions.push(condition);
    }
    return this;
  }

  whereIn(field: string, values: any[]): this {
    if (values.length === 0) return this;
    
    const placeholders = values.map(() => `$${this.paramIndex++}`).join(', ');
    this.whereConditions.push(`${field} IN (${placeholders})`);
    this.whereParams.push(...values);
    return this;
  }

  whereBetween(field: string, start: any, end: any): this {
    this.whereConditions.push(`${field} BETWEEN $${this.paramIndex++} AND $${this.paramIndex++}`);
    this.whereParams.push(start, end);
    return this;
  }

  whereILike(field: string, pattern: string): this {
    this.whereConditions.push(`${field} ILIKE $${this.paramIndex++}`);
    this.whereParams.push(`%${pattern}%`);
    return this;
  }

  join(table: string, condition: string): this {
    this.joinClauses.push(`JOIN ${table} ON ${condition}`);
    return this;
  }

  leftJoin(table: string, condition: string): this {
    this.joinClauses.push(`LEFT JOIN ${table} ON ${condition}`);
    return this;
  }

  orderBy(field: string, direction: 'ASC' | 'DESC' = 'ASC'): this {
    this.orderByClause = `ORDER BY ${field} ${direction}`;
    return this;
  }

  limit(count: number): this {
    this.limitClause = `LIMIT ${count}`;
    return this;
  }

  offset(count: number): this {
    this.offsetClause = `OFFSET ${count}`;
    return this;
  }

  build(): { sql: string; params: any[] } {
    const parts = [
      `SELECT ${this.selectFields.join(', ')}`,
      `FROM ${this.tableName}`,
      ...this.joinClauses,
    ];

    if (this.whereConditions.length > 0) {
      parts.push(`WHERE ${this.whereConditions.join(' AND ')}`);
    }

    if (this.orderByClause) {
      parts.push(this.orderByClause);
    }

    if (this.limitClause) {
      parts.push(this.limitClause);
    }

    if (this.offsetClause) {
      parts.push(this.offsetClause);
    }

    return {
      sql: parts.join(' '),
      params: this.whereParams,
    };
  }

  async execute(pool: Pool): Promise<any[]> {
    const { sql, params } = this.build();
    const result = await pool.query(sql, params);
    return result.rows;
  }

  async count(pool: Pool): Promise<number> {
    const countBuilder = new ApiQueryBuilder(this.tableName);
    countBuilder.selectFields = ['COUNT(*) as total'];
    countBuilder.whereConditions = [...this.whereConditions];
    countBuilder.whereParams = [...this.whereParams];
    countBuilder.joinClauses = [...this.joinClauses];
    
    const { sql, params } = countBuilder.build();
    const result = await pool.query(sql, params);
    return parseInt(result.rows[0].total, 10);
  }
}

// Pagination utility
export async function buildPaginatedQuery<T>(
  pool: Pool,
  baseQuery: ApiQueryBuilder,
  pagination: PaginationQuery
): Promise<PaginationResponse<T>> {
  const { limit, offset, sort, order } = pagination;

  // Apply sorting if specified
  if (sort) {
    baseQuery.orderBy(sort, order.toUpperCase() as 'ASC' | 'DESC');
  }

  // Get total count
  const total = await baseQuery.count(pool);

  // Apply pagination
  baseQuery.limit(limit).offset(offset);

  // Execute query
  const items = await baseQuery.execute(pool) as T[];

  // Calculate pagination metadata
  const hasMore = offset + limit < total;
  const nextCursor = hasMore ? btoa(`offset:${offset + limit}`) : undefined;
  const prevCursor = offset > 0 ? btoa(`offset:${Math.max(0, offset - limit)}`) : undefined;

  return {
    items,
    pagination: {
      total,
      limit,
      offset,
      hasMore,
      nextCursor,
      prevCursor,
    },
  };
}

// Search utility for full-text search
export function buildSearchQuery(
  baseQuery: ApiQueryBuilder,
  searchFields: string[],
  searchTerm: string
): ApiQueryBuilder {
  if (!searchTerm || searchFields.length === 0) {
    return baseQuery;
  }

  const searchConditions = searchFields.map(field => `${field} ILIKE ?`);
  const searchPattern = `%${searchTerm}%`;
  
  // Use OR conditions for search across multiple fields
  const combinedCondition = `(${searchConditions.join(' OR ')})`;
  
  // Add the same parameter for each field
  searchFields.forEach(() => {
    baseQuery.where(combinedCondition, searchPattern);
  });

  return baseQuery;
}

// Filter builder for complex filtering
export function applyFilters(
  query: ApiQueryBuilder,
  filters: Record<string, any>
): ApiQueryBuilder {
  for (const [key, value] of Object.entries(filters)) {
    if (value === undefined || value === null) continue;

    switch (key) {
      case 'created_range':
      case 'updated_range':
      case 'started_range':
      case 'completed_range':
        if (value.from || value.to) {
          const field = key.replace('_range', '_at');
          if (value.from && value.to) {
            query.whereBetween(field, value.from, value.to);
          } else if (value.from) {
            query.where(`${field} >= ?`, value.from);
          } else if (value.to) {
            query.where(`${field} <= ?`, value.to);
          }
        }
        break;

      case 'tags':
        if (Array.isArray(value) && value.length > 0) {
          // PostgreSQL array overlap operator
          query.where('tags && ?', `{${value.join(',')}}`);
        }
        break;

      case 'search':
        if (value.query && value.fields) {
          buildSearchQuery(query, value.fields, value.query);
        }
        break;

      default:
        if (Array.isArray(value)) {
          query.whereIn(key, value);
        } else if (typeof value === 'boolean') {
          query.where(`${key} = ?`, value);
        } else if (typeof value === 'string' || typeof value === 'number') {
          query.where(`${key} = ?`, value);
        }
        break;
    }
  }

  return query;
}

// Database transaction wrapper
export async function withTransaction<T>(
  pool: Pool,
  callback: (client: any) => Promise<T>
): Promise<T> {
  const client = await pool.connect();
  
  try {
    await client.query('BEGIN');
    const result = await callback(client);
    await client.query('COMMIT');
    return result;
  } catch (error) {
    await client.query('ROLLBACK');
    throw error;
  } finally {
    client.release();
  }
}

// Optimized exists check
export async function recordExists(
  pool: Pool,
  tableName: string,
  conditions: Record<string, any>
): Promise<boolean> {
  const query = new ApiQueryBuilder(tableName)
    .select('1');

  for (const [key, value] of Object.entries(conditions)) {
    query.where(`${key} = ?`, value);
  }

  query.limit(1);

  const { sql, params } = query.build();
  const result = await pool.query(sql, params);
  return result.rows.length > 0;
}

// Batch insert utility
export async function batchInsert(
  pool: Pool,
  tableName: string,
  records: Record<string, any>[],
  batchSize: number = 100
): Promise<void> {
  if (records.length === 0) return;

  const fields = Object.keys(records[0]);
  const placeholders = fields.map((_, i) => `$${i + 1}`).join(', ');
  
  for (let i = 0; i < records.length; i += batchSize) {
    const batch = records.slice(i, i + batchSize);
    const values: any[] = [];
    const valueRows: string[] = [];
    
    batch.forEach((record, recordIndex) => {
      const recordPlaceholders = fields.map((field, fieldIndex) => {
        values.push(record[field]);
        return `$${recordIndex * fields.length + fieldIndex + 1}`;
      });
      valueRows.push(`(${recordPlaceholders.join(', ')})`);
    });

    const sql = `
      INSERT INTO ${tableName} (${fields.join(', ')})
      VALUES ${valueRows.join(', ')}
    `;

    await pool.query(sql, values);
  }
}

// Upsert utility (INSERT ... ON CONFLICT)
export async function upsert(
  pool: Pool,
  tableName: string,
  record: Record<string, any>,
  conflictFields: string[],
  updateFields?: string[]
): Promise<any> {
  const fields = Object.keys(record);
  const values = Object.values(record);
  const placeholders = fields.map((_, i) => `$${i + 1}`).join(', ');
  
  const fieldsToUpdate = updateFields || fields.filter(f => !conflictFields.includes(f));
  const updateClause = fieldsToUpdate
    .map(field => `${field} = EXCLUDED.${field}`)
    .join(', ');

  const sql = `
    INSERT INTO ${tableName} (${fields.join(', ')})
    VALUES (${placeholders})
    ON CONFLICT (${conflictFields.join(', ')}) 
    DO UPDATE SET ${updateClause}
    RETURNING *
  `;

  const result = await pool.query(sql, values);
  return result.rows[0];
}

// Performance monitoring for queries
export async function queryWithMetrics<T>(
  pool: Pool,
  sql: string,
  params: any[] = []
): Promise<{ result: T; metrics: { duration: number; rows: number } }> {
  const startTime = Date.now();
  
  try {
    const result = await pool.query(sql, params);
    const duration = Date.now() - startTime;
    
    // Log slow queries
    if (duration > 1000) {
      console.warn(`Slow query detected (${duration}ms):`, { sql, params, rows: result.rows.length });
    }
    
    return {
      result: result.rows as T,
      metrics: {
        duration,
        rows: result.rows.length,
      },
    };
  } catch (error) {
    const duration = Date.now() - startTime;
    console.error(`Query failed after ${duration}ms:`, { sql, params, error });
    throw error;
  }
}