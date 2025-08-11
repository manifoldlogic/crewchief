import { Server } from 'json-rpc-2.0';
import { query } from './db'; // Assuming db.ts

const server = new Server();

server.addMethod('search', async (params) => {
  // TODO: Implement search logic
  return { hits: [] };
});

// Add other methods: context, open, upsert, explain

process.stdin.on('data', (chunk) => {
  const request = chunk.toString();
  server.receive(request).then((response) => {
    if (response) {
      console.log(response);
    }
  });
});
