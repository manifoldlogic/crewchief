
import { spawn } from 'child_process';
import { v4 as uuidv4 } from 'uuid';
import { getDatabase } from '../db/connection';

class TmuxService {
  async createSession(name) {
    const id = uuidv4();
    spawn('tmux', ['new-session', '-d', '-s', id]);
    await getDatabase().query('INSERT INTO tmux_sessions (id, name) VALUES ($1, $2)', [id, name]);
    return id;
  }

  // Additional methods

}

export const tmuxService = new TmuxService();
