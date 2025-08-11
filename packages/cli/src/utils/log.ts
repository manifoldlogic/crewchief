import fs from 'node:fs';

export function tailFileSync(filePath: string, maxLines: number = 100): string {
  if (!fs.existsSync(filePath)) return '';
  const data = fs.readFileSync(filePath, 'utf8');
  const lines = data.split('\n');
  const tail = lines.slice(Math.max(0, lines.length - maxLines));
  return tail.join('\n');
}


