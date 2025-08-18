import type { DatabaseConnection } from '../../../db/connection.js';
import { QueryBuilder } from '../../../db/query-builder.js';

export interface PaginationArgs {
  limit?: number;
  offset?: number;
  page?: number;
  pageSize?: number;
}

export interface SortArgs {
  field: string;
  direction: 'ASC' | 'DESC';
}

export interface ConnectionResult<T> {
  edges: Array<{
    node: T;
    cursor: string;
  }>;
  pageInfo: {
    hasNextPage: boolean;
    hasPreviousPage: boolean;
    startCursor?: string;
    endCursor?: string;
    totalCount: number;
    pageSize: number;
    page: number;
  };
}

export class DatabaseService {
  constructor(private db: DatabaseConnection) {}

  async executeQuery<T = any>(query: string, params: any[] = []): Promise<T[]> {
    try {
      const result = await this.db.query(query, params);
      return result.rows;
    } catch (error) {
      console.error('Database query error:', error);
      throw new Error(`Database query failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  async executeQuerySingle<T = any>(query: string, params: any[] = []): Promise<T | null> {
    const results = await this.executeQuery<T>(query, params);
    return results.length > 0 ? results[0] : null;
  }

  async getConnection<T>(
    tableName: string,
    filter?: Record<string, any>,
    sort?: SortArgs,
    pagination?: PaginationArgs
  ): Promise<ConnectionResult<T>> {
    const qb = new QueryBuilder(tableName);

    // Apply filters
    if (filter) {
      Object.entries(filter).forEach(([key, value]) => {
        if (value !== undefined && value !== null) {
          if (Array.isArray(value)) {
            qb.whereIn(key, value);
          } else if (typeof value === 'string' && key === 'search') {
            // Handle search across multiple fields
            qb.where(`(name ILIKE ? OR description ILIKE ?)`, [`%${value}%`, `%${value}%`]);
          } else {
            qb.where(`${key} = ?`, value);
          }
        }
      });
    }

    // Get total count for pagination
    const countQuery = qb.clone().count();
    const totalCountResult = await this.executeQuery(countQuery.query, countQuery.params);
    const totalCount = parseInt(totalCountResult[0].count);

    // Apply sorting
    if (sort) {
      qb.orderBy(sort.field, sort.direction);
    } else {
      qb.orderBy('created_at', 'DESC');
    }

    // Apply pagination
    const limit = pagination?.pageSize || pagination?.limit || 50;
    const page = pagination?.page || 1;
    const offset = pagination?.offset || (page - 1) * limit;

    qb.limit(limit).offset(offset);

    // Execute query
    const query = qb.build();
    const results = await this.executeQuery<T>(query.query, query.params);

    // Build connection result
    const edges = results.map((node, index) => ({
      node,
      cursor: Buffer.from(`${offset + index}`).toString('base64'),
    }));

    const hasNextPage = offset + limit < totalCount;
    const hasPreviousPage = offset > 0;

    return {
      edges,
      pageInfo: {
        hasNextPage,
        hasPreviousPage,
        startCursor: edges.length > 0 ? edges[0].cursor : undefined,
        endCursor: edges.length > 0 ? edges[edges.length - 1].cursor : undefined,
        totalCount,
        pageSize: limit,
        page,
      },
    };
  }

  // Utility method for creating standardized responses
  createResponse<T>(success: boolean, data?: T, errors?: Array<{ message: string; code?: string; field?: string }>): {
    success: boolean;
    errors: Array<{ message: string; code?: string; field?: string }>;
  } & T {
    return {
      success,
      errors: errors || [],
      ...data,
    } as any;
  }

  // Validation helpers
  validateRequired(fields: Record<string, any>, requiredFields: string[]): Array<{ field: string; message: string; code: string }> {
    const errors: Array<{ field: string; message: string; code: string }> = [];
    
    requiredFields.forEach(field => {
      if (!fields[field] || (typeof fields[field] === 'string' && fields[field].trim() === '')) {
        errors.push({
          field,
          message: `${field} is required`,
          code: 'REQUIRED_FIELD_MISSING',
        });
      }
    });

    return errors;
  }

  // Transaction helper
  async withTransaction<T>(callback: (db: DatabaseConnection) => Promise<T>): Promise<T> {
    return await this.db.transaction(async (client) => {
      // For now, pass the database connection to the callback
      // In a more advanced implementation, we might wrap the client
      return await callback(this.db);
    });
  }
}

// Singleton instance
let dbService: DatabaseService | null = null;

export function initializeDatabaseService(db: DatabaseConnection): DatabaseService {
  dbService = new DatabaseService(db);
  return dbService;
}

export function getDatabaseService(): DatabaseService {
  if (!dbService) {
    throw new Error('Database service not initialized. Call initializeDatabaseService first.');
  }
  return dbService;
}