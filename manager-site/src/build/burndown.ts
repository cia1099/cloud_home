import { readText, repoPath } from './util.ts';

export interface BurnPoint {
  date: string;
  open: number;
}

export interface Burndown {
  points: BurnPoint[];
  /** Only set when the CSV carries a "# target=YYYY-MM-DD" line; without it no ideal-pace line is drawn. */
  target?: string;
}

/** plan/burndown.csv: optional "# target=YYYY-MM-DD", then "date,done,total,dropped" rows, one per day. */
export function parseBurndown(text: string | null): Burndown {
  const points: BurnPoint[] = [];
  let target: string | undefined;
  for (const [i, raw] of (text ?? '').split('\n').entries()) {
    const line = raw.trim();
    if (!line || line.startsWith('date,')) continue;
    const t = /^#\s*target=(\d{4}-\d{2}-\d{2})$/.exec(line);
    if (t) {
      target = t[1];
      continue;
    }
    if (line.startsWith('#')) continue;
    const m = /^(\d{4}-\d{2}-\d{2}),(\d+),(\d+),(\d+)$/.exec(line);
    if (!m) throw new Error(`plan/burndown.csv line ${i + 1}: expected "date,done,total,dropped"`);
    const [done, total, dropped] = [Number(m[2]), Number(m[3]), Number(m[4])];
    points.push({ date: m[1] ?? '', open: total - dropped - done });
  }
  points.sort((a, b) => a.date.localeCompare(b.date));
  return { points, target };
}

export const loadBurndown = (): Burndown => parseBurndown(readText(repoPath('plan/burndown.csv')));
