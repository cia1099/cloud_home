import { ROLES, STATUSES, type Bi, type Role, type Status } from '../shared/i18n.ts';
import { asBi, asObject, readJson, readText, repoPath, SITE_ROOT, warn } from './util.ts';
import { resolve } from 'node:path';

export interface Requirement {
  id: string;
  pri: 'P0' | 'P1' | 'P2';
  status: Status;
  area: string;
  title: Bi;
  /** English reason shown under a blocked row, taken from the "Blocked: ..." sentence in SPEC.md. */
  blocked?: Bi;
  owner: Role | null;
}

export interface Area {
  key: string;
  name: Bi;
  reqs: Requirement[];
}

export interface SpecChange {
  date: string;
  text: string;
}

export interface Spec {
  areas: Area[];
  reqs: Requirement[];
  changes: SpecChange[];
}

export type Counts = Record<Status, number>;

export const emptyCounts = (): Counts => ({ todo: 0, doing: 0, review: 0, done: 0, blocked: 0, dropped: 0 });

export function count(reqs: Requirement[]): Counts {
  const c = emptyCounts();
  for (const r of reqs) c[r.status]++;
  return c;
}

/** done / (total - dropped), as in design/system.md section 3. */
export const planned = (c: Counts): number => STATUSES.reduce((n, s) => n + (s === 'dropped' ? 0 : c[s]), 0);

const splitRow = (line: string): string[] =>
  line
    .split(/(?<!\\)\|/)
    .slice(1, -1)
    .map((c) => c.trim().replace(/\\\|/g, '|'));

interface AreaDef {
  match: string;
  key: string;
  name: Bi;
}

export function parseSpec(text: string, areaDefs: AreaDef[], zhTitles: Record<string, { title?: string; blocked?: string }>): Spec {
  const lines = text.split('\n');
  const areas: Area[] = [];
  const changes: SpecChange[] = [];
  let section = '';
  let area: Area | null = null;
  for (const line of lines) {
    const h2 = /^## (\d+)\./.exec(line);
    if (h2) {
      section = h2[1] ?? '';
      area = null;
      continue;
    }
    if (section === '3') {
      const h3 = /^### (.+)$/.exec(line);
      if (h3) {
        const heading = h3[1] ?? '';
        const def = areaDefs.find((d) => heading.startsWith(d.match));
        if (!def) throw new Error(`SPEC.md heading "${heading}" has no entry in manager-site/content/areas.json`);
        area = { key: def.key, name: def.name, reqs: [] };
        areas.push(area);
        continue;
      }
      if (area && /^\| R-\d+ /.test(line)) area.reqs.push(parseRow(line, area.key, zhTitles));
    } else if (section === '7') {
      const cells = /^\| (\d{4}-\d{2}-\d{2}) \|/.test(line) ? splitRow(line) : null;
      if (cells) changes.push({ date: cells[0] ?? '', text: cells[1] ?? '' });
    }
  }
  const reqs = areas.flatMap((a) => a.reqs);
  const ids = new Set<string>();
  for (const r of reqs) {
    if (ids.has(r.id)) throw new Error(`SPEC.md: duplicate requirement id ${r.id}`);
    ids.add(r.id);
  }
  if (!reqs.length) throw new Error('SPEC.md: no requirement rows found in section 3');
  return { areas, reqs, changes: changes.reverse() };
}

function parseRow(line: string, area: string, zhTitles: Record<string, { title?: string; blocked?: string }>): Requirement {
  const [id = '', pri = '', status = '', body = '', ownerCell = ''] = splitRow(line);
  if (pri !== 'P0' && pri !== 'P1' && pri !== 'P2') throw new Error(`SPEC.md ${id}: priority "${pri}" is not P0/P1/P2`);
  if (!(STATUSES as readonly string[]).includes(status)) throw new Error(`SPEC.md ${id}: unknown status "${status}"`);
  const title = /\*\*(.+?)\*\*/.exec(body)?.[1]?.replace(/[`*]/g, '').replace(/\s*\.$/, '');
  if (!title) throw new Error(`SPEC.md ${id}: the requirement text must start with a **bold title**`);
  const zh = zhTitles[id];
  if (!zh?.title) warn(`no Chinese title for ${id} in content/requirements.zh.json (English is shown in both languages)`);
  let blocked: Bi | undefined;
  if (status === 'blocked') {
    const en = /Blocked:\s*(.+?\.)(?=\s|$)/.exec(body)?.[1]?.trim() ?? 'Waiting for something that is not recorded in SPEC.md.';
    blocked = { en, zh: zh?.blocked ?? en };
    if (!zh?.blocked) warn(`no Chinese blocked reason for ${id} in content/requirements.zh.json`);
  }
  const ownerWord = ownerCell.toLowerCase().match(/manager|backend|frontend|designer|tester/)?.[0] as Role | undefined;
  return {
    id,
    pri,
    status: status as Status,
    area,
    title: { en: title, zh: zh?.title ?? title },
    blocked,
    owner: ownerWord && (ROLES as readonly string[]).includes(ownerWord) ? ownerWord : null,
  };
}

export function loadSpec(): Spec {
  const text = readText(repoPath('SPEC.md'));
  if (text === null) throw new Error('SPEC.md not found at the repository root');
  const areaDefs = (readJson(resolve(SITE_ROOT, 'content/areas.json')) as unknown[]).map((v, i) => {
    const o = asObject(v, `areas.json[${i}]`);
    return { match: String(o.match), key: String(o.key), name: asBi(o.name, `areas.json[${i}].name`) };
  });
  const zh = asObject(readJson(resolve(SITE_ROOT, 'content/requirements.zh.json')), 'requirements.zh.json');
  return parseSpec(text, areaDefs, zh as Record<string, { title?: string; blocked?: string }>);
}
