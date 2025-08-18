import type { ApolloServerPlugin } from '@apollo/server';
import depthLimit from 'graphql-depth-limit';

export function createDepthLimitPlugin(maxDepth: number = 10): ApolloServerPlugin {
  return {
    requestDidStart() {
      return {
        didResolveOperation({ request, document }) {
          const depthLimitRule = depthLimit(maxDepth);
          const errors = depthLimitRule.validation?.(
            { validate: () => [] } as any,
            document,
            {} as any
          );
          
          if (errors && errors.length > 0) {
            throw new Error(`Query depth limit of ${maxDepth} exceeded`);
          }
        },
      };
    },
  };
}