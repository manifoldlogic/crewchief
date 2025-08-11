import pgp from 'pg-promise';

const pgpInstance = pgp();
const db = pgpInstance(process.env.DATABASE_URL || 'postgres://maproom_writer:secret@localhost:5432/maproom');

export async function query(sql: string, params: any[] = []) {
  try {
    return await db.any(sql, params);
  } catch (error) {
    console.error('DB query error:', error);
    throw error;
  }
}

// Example usage:
// const repos = await query('SELECT * FROM maproom.repos');
