import { spawn } from 'node:child_process';

export interface RunCommandResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

export interface RunCommandOptions {
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  input?: string;
  /** When true, streams child stdout/stderr to parent process in real-time */
  inheritStreams?: boolean;
  /** Kill process after ms if still running */
  timeoutMs?: number;
}

export function runCommand(command: string, args: string[] = [], options: RunCommandOptions = {}): Promise<RunCommandResult> {
  return new Promise<RunCommandResult>((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd ?? process.cwd(),
      env: { ...process.env, ...options.env },
      stdio: ['pipe', 'pipe', 'pipe']
    });

    let stdout = '';
    let stderr = '';
    let timeout: NodeJS.Timeout | undefined;

    if (options.timeoutMs && options.timeoutMs > 0) {
      timeout = setTimeout(() => {
        child.kill('SIGKILL');
        reject(new Error(`Command timed out after ${options.timeoutMs}ms: ${command} ${args.join(' ')}`));
      }, options.timeoutMs);
    }

    if (options.input) {
      child.stdin.write(options.input);
      child.stdin.end();
    }

    child.stdout.on('data', (chunk: Buffer) => {
      const text = chunk.toString();
      stdout += text;
      if (options.inheritStreams) process.stdout.write(text);
    });

    child.stderr.on('data', (chunk: Buffer) => {
      const text = chunk.toString();
      stderr += text;
      if (options.inheritStreams) process.stderr.write(text);
    });

    child.on('error', (err) => {
      if (timeout) clearTimeout(timeout);
      reject(err);
    });

    child.on('close', (code) => {
      if (timeout) clearTimeout(timeout);
      const exitCode = code ?? -1;
      resolve({ exitCode, stdout, stderr });
    });
  });
}


