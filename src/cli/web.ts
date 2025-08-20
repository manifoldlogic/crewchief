import { Command } from 'commander';
import { spawn } from 'child_process';
import open from 'open';

const webCommand = new Command('web')
  .description('Start CrewChief Web UI')
  .action(async () => {
    const port = process.env.PORT || 3456;
    const server = spawn('node', ['dist/server.js'], { env: { ...process.env, PORT: port.toString() } });
    server.on('error', (err) => console.error('Server error:', err));
    await open(`http://localhost:${port}`);
  });

export default webCommand;
