import { expect, test } from 'bun:test';
import { parseApprovals } from './approvals.ts';
import { parseBurndown } from './burndown.ts';
import { count, parseSpec, planned } from './spec.ts';
import { parseStatus } from './status.ts';

const AREAS = [{ match: 'Backend', key: 'backend', name: { en: 'Backend', zh: '后端' } }];

const SPEC = `# SPEC
## 3. Requirements
### Backend
| ID | Pri | Status | Requirement | Owner |
|---|---|---|---|---|
| R-001 | P0 | done | **First thing.** Details | backend-engineer |
| R-002 | P1 | blocked | **Second** text. Blocked: needs the owner. More. | manager → designer |
| R-003 | P2 | dropped | **Third** with a\\|pipe | tester |
## 7. Change log
| Date | Change |
|---|---|
| 2026-11-19 | First. |
| 2026-11-20 | Second. |
`;

test('spec rows, owners, blocked reason, change log order', () => {
  const s = parseSpec(SPEC, AREAS, { 'R-001': { title: '第一件' } });
  expect(s.reqs.map((r) => r.id)).toEqual(['R-001', 'R-002', 'R-003']);
  expect(s.reqs[0]?.title).toEqual({ en: 'First thing', zh: '第一件' });
  expect(s.reqs[1]?.blocked?.en).toBe('needs the owner.');
  expect(s.reqs[1]?.owner).toBe('manager');
  expect(s.changes[0]?.date).toBe('2026-11-20');
  const c = count(s.reqs);
  expect([c.done, c.blocked, c.dropped, planned(c)]).toEqual([1, 1, 1, 2]);
});

test('spec rejects an unknown area heading', () => {
  expect(() => parseSpec(SPEC.replace('### Backend', '### Mystery'), AREAS, {})).toThrow('areas.json');
});

const APPROVALS = `## Request format
\`\`\`
### A-001 [ ] <short title>
\`\`\`
## Requests
### A-003 [ ] Tag v0.3.1 — 2026-12-02 — requested by manager
- What: Push and tag.
- If approved: Pipeline runs.   If denied: Stay on rc.
- zh title: 打标签
- zh What: 推送并打标签。
### A-002 [!] Paid SMS — 2026-11-26
- Owner note: email first
`;

test('approvals: pending, denied, zh fallbacks, fenced template ignored', () => {
  const a = parseApprovals(APPROVALS);
  expect(a.map((x) => [x.id, x.state])).toEqual([['A-003', 'pending'], ['A-002', 'denied']]);
  expect(a[0]?.title).toEqual({ en: 'Tag v0.3.1', zh: '打标签' });
  expect(a[0]?.fields.what).toEqual({ en: 'Push and tag.', zh: '推送并打标签。' });
  expect(a[0]?.fields.ifDenied?.en).toBe('Stay on rc.');
  expect(a[1]?.fields.note?.zh).toBe('email first');
});

test('burn-down: open count, optional target, no invented points', () => {
  expect(parseBurndown(null).points).toEqual([]);
  const b = parseBurndown('# target=2026-12-31\ndate,done,total,dropped\n2026-11-25,2,35,1\n2026-11-24,0,35,0\n');
  expect(b.target).toBe('2026-12-31');
  expect(b.points).toEqual([{ date: '2026-11-24', open: 35 }, { date: '2026-11-25', open: 32 }]);
  expect(parseBurndown('date,done,total,dropped\n2026-11-24,0,35,0\n').target).toBeUndefined();
  expect(() => parseBurndown('nonsense')).toThrow('burndown.csv');
});

test('status block: missing block is empty, bad JSON names the file', () => {
  expect(parseStatus('# nothing').roles).toEqual({});
  const s = parseStatus('```json site-status\n{"roles":{"tester":{"working":true,"task":{"en":"a","zh":"b"}}}}\n```');
  expect(s.roles.tester?.working).toBe(true);
  expect(() => parseStatus('```json site-status\n{oops\n```')).toThrow('STATUS.md');
});
