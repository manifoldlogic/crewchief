#!/usr/bin/env node
// Simple mock agent: emits JSONL status lines and echoes stdin as result
// Also emits Ops Deck-compatible heartbeat lines to stdout (prefixed for easy parsing if needed)

function envelope(type, payload) {
  return JSON.stringify({ id: Math.random().toString(36).slice(2), type, payload, ts: new Date().toISOString() }) + '\n';
}

function heartbeat(agentId, state) {
  const hb = {
    agent_id: agentId,
    ts: new Date().toISOString(),
    state
  };
  return JSON.stringify(hb) + '\n';
}

const agentId = process.env.MOCK_AGENT_ID || Math.random().toString(36).slice(2);
process.stdout.write(envelope('status', { message: 'agent started', agentId }));
process.stdout.write(heartbeat(agentId, 'RUNNING'));

let count = 0;
const interval = setInterval(() => {
  count += 1;
  process.stdout.write(envelope('status', { heartbeat: count, agentId }));
  process.stdout.write(heartbeat(agentId, 'RUNNING'));
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


