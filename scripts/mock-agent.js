#!/usr/bin/env node
// Simple mock agent: emits JSONL status lines and echoes stdin as result

function envelope(type, payload) {
  return JSON.stringify({ id: Math.random().toString(36).slice(2), type, payload, ts: new Date().toISOString() }) + '\n';
}

process.stdout.write(envelope('status', { message: 'agent started' }));

let count = 0;
const interval = setInterval(() => {
  count += 1;
  process.stdout.write(envelope('status', { heartbeat: count }));
  if (count >= 5) {
    clearInterval(interval);
  }
}, 500);

process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
  const msg = chunk.toString().trim();
  if (msg) {
    process.stdout.write(envelope('result', { echo: msg }));
  }
});


