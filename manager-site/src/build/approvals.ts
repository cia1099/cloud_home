import type { Bi } from '../shared/i18n.ts';
import { readText, repoPath } from './util.ts';

export interface Approval {
  id: string;
  state: 'pending' | 'approved' | 'denied';
  title: Bi;
  requested?: string;
  fields: { what?: Bi; why?: Bi; risk?: Bi; ifApproved?: Bi; ifDenied?: Bi; note?: Bi };
}

const MARK = { ' ': 'pending', x: 'approved', '!': 'denied' } as const;
const FIELD: Record<string, keyof Approval['fields']> = {
  what: 'what',
  why: 'why',
  risk: 'risk',
  'if approved': 'ifApproved',
  'if denied': 'ifDenied',
  'owner note': 'note',
};

/**
 * plan/APPROVALS.md, section "## Requests":
 *   ### A-003 [ ] English title — 2026-12-02 — requested by manager
 *   - What: ...   - Why: ...   - Risk: ...   - If approved: ...   If denied: ...   - Owner note: ...
 * Optional Chinese lines use the same keys with a "zh " prefix ("- zh title:", "- zh What:", ...).
 * Heading marks: [ ] pending, [x] approved, [!] denied. Fenced code blocks are ignored.
 */
export function parseApprovals(text: string): Approval[] {
  const out: Approval[] = [];
  let inRequests = false;
  let fenced = false;
  let cur: { a: Approval; zh: Record<string, string>; en: Record<string, string>; zhTitle?: string } | null = null;
  const flush = (): void => {
    if (!cur) return;
    const { a, en, zh } = cur;
    for (const [k, f] of Object.entries(FIELD)) {
      const e = en[k];
      if (e) a.fields[f] = { en: e, zh: zh[k] ?? e };
    }
    a.title = { en: a.title.en, zh: cur.zhTitle ?? a.title.en };
    out.push(a);
    cur = null;
  };
  for (const raw of text.split('\n')) {
    const line = raw.trimEnd();
    if (line.startsWith('```')) {
      fenced = !fenced;
      continue;
    }
    if (fenced) continue;
    if (/^## /.test(line)) {
      flush();
      inRequests = /^## Requests/.test(line);
      continue;
    }
    if (!inRequests) continue;
    const h = /^### (A-\d+) \[([ x!])\] (.+)$/.exec(line);
    if (h) {
      flush();
      const parts = (h[3] ?? '').split(' — ');
      const title = parts[0]?.trim() ?? '';
      const requested = parts.find((p) => /^\d{4}-\d{2}-\d{2}$/.test(p.trim()))?.trim();
      cur = {
        a: { id: h[1] ?? '', state: MARK[(h[2] ?? ' ') as keyof typeof MARK], title: { en: title, zh: title }, requested, fields: {} },
        zh: {},
        en: {},
      };
      continue;
    }
    const b = /^- (zh )?([A-Za-z ]+?|title):\s*(.*)$/.exec(line);
    if (cur && b) {
      const key = (b[2] ?? '').toLowerCase();
      let value = (b[3] ?? '').trim();
      if (key === 'if approved' && /\s{2,}If denied:/.test(value)) {
        const [ap = '', den = ''] = value.split(/\s{2,}If denied:\s*/);
        (b[1] ? cur.zh : cur.en)['if denied'] = den.trim();
        value = ap.trim();
      }
      if (key === 'title' && b[1]) cur.zhTitle = value;
      else if (b[1]) cur.zh[key] = value;
      else cur.en[key] = value;
    }
  }
  flush();
  return out;
}

export function loadApprovals(): Approval[] {
  return parseApprovals(readText(repoPath('plan/APPROVALS.md')) ?? '');
}
