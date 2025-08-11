import { spawn } from 'node:child_process';
import path from 'node:path';
import { AgentType } from './types';

export interface AgentStartOptions {
  cwd: string;
  env?: NodeJS.ProcessEnv;
  args?: string[];
}

export class AgentRunner {
  start(type: AgentType, opts: AgentStartOptions) {
    const shell = process.env.SHELL || '/bin/bash';
    const args = ['-lc', `${type.executionCommand} ${[...(opts.args ?? [])].join(' ')}`];
    const pty = spawn(shell, args, {
      name: 'xterm-color',
      cols: 80,
      rows: 30,
      cwd: opts.cwd,
      env: { ...process.env, ...opts.env }
    });
    return pty;
  }
}


