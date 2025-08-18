import { GraphQLScalarType } from 'graphql';
import { Kind } from 'graphql/language';

// Custom scalar for DateTime (ISO 8601 strings)
export const DateTime = new GraphQLScalarType({
  name: 'DateTime',
  description: 'DateTime custom scalar type (ISO 8601)',
  serialize(value: unknown): string {
    if (value instanceof Date) {
      return value.toISOString();
    }
    if (typeof value === 'string') {
      return new Date(value).toISOString();
    }
    throw new Error(`Value is not a valid DateTime: ${value}`);
  },
  parseValue(value: unknown): Date {
    if (typeof value !== 'string') {
      throw new Error(`Value is not a string: ${value}`);
    }
    const date = new Date(value);
    if (isNaN(date.getTime())) {
      throw new Error(`Value is not a valid DateTime string: ${value}`);
    }
    return date;
  },
  parseLiteral(ast): Date {
    if (ast.kind !== Kind.STRING) {
      throw new Error(`Can only parse strings to DateTime but got a: ${ast.kind}`);
    }
    const date = new Date(ast.value);
    if (isNaN(date.getTime())) {
      throw new Error(`Value is not a valid DateTime string: ${ast.value}`);
    }
    return date;
  },
});

// Custom scalar for JSON (JSONB fields)
export const JSON = new GraphQLScalarType({
  name: 'JSON',
  description: 'JSON custom scalar type',
  serialize(value: unknown): unknown {
    return value;
  },
  parseValue(value: unknown): unknown {
    return value;
  },
  parseLiteral(ast): unknown {
    switch (ast.kind) {
      case Kind.STRING:
        try {
          return JSON.parse(ast.value);
        } catch {
          throw new Error(`Invalid JSON string: ${ast.value}`);
        }
      case Kind.OBJECT:
        return parseObjectLiteral(ast);
      case Kind.LIST:
        return ast.values.map(parseLiteral);
      case Kind.INT:
        return parseInt(ast.value, 10);
      case Kind.FLOAT:
        return parseFloat(ast.value);
      case Kind.BOOLEAN:
        return ast.value;
      case Kind.NULL:
        return null;
      default:
        throw new Error(`Unexpected kind in JSON literal: ${ast.kind}`);
    }
  },
});

// Helper function to parse object literals
function parseObjectLiteral(ast: any): any {
  const value: any = {};
  ast.fields.forEach((field: any) => {
    value[field.name.value] = parseLiteral(field.value);
  });
  return value;
}

// Helper function to parse literals recursively
function parseLiteral(ast: any): any {
  switch (ast.kind) {
    case Kind.STRING:
      return ast.value;
    case Kind.OBJECT:
      return parseObjectLiteral(ast);
    case Kind.LIST:
      return ast.values.map(parseLiteral);
    case Kind.INT:
      return parseInt(ast.value, 10);
    case Kind.FLOAT:
      return parseFloat(ast.value);
    case Kind.BOOLEAN:
      return ast.value;
    case Kind.NULL:
      return null;
    default:
      throw new Error(`Unexpected kind in literal: ${ast.kind}`);
  }
}