import type { Bi } from '../shared/i18n.ts';
import { loadApprovals, type Approval } from './approvals.ts';
import { loadBurndown, type Burndown } from './burndown.ts';
import { loadGit, type GitCommit } from './git.ts';
import { loadReports, type Report, type Tests } from './reports.ts';
import { count, loadSpec, type Counts, type Spec } from './spec.ts';
import { loadStatus, type SiteStatus } from './status.ts';

export type EventType = 'commit' | 'release' | 'report' | 'decision' | 'spec';

export interface SiteEvent {
  at: string;
  type: EventType;
  title: Bi;
  hash?: string;
  tag?: string;
  /** a report id (opens that report) or an A-nnn / R-nnn id */
  ref?: string;
}

export interface Site {
  spec: Spec;
  counts: Counts;
  status: SiteStatus;
  approvals: Approval[];
  pending: Approval[];
  burndown: Burndown;
  commits: GitCommit[];
  head: string | null;
  latestTag: string | null;
  reports: Report[];
  latest: Report | undefined;
  /** the newest tests.json found in any report */
  tests: { tests: Tests; report: Report } | undefined;
  events: SiteEvent[];
  builtAt: Date;
}

const sum = (t: Tests, k: 'passed' | 'failed' | 'skipped'): number => t.suites.reduce((n, s) => n + s[k], 0);
export const testTotals = (t: Tests): { passed: number; failed: number; skipped: number } => ({ passed: sum(t, 'passed'), failed: sum(t, 'failed'), skipped: sum(t, 'skipped') });

function buildEvents(commits: GitCommit[], reports: Report[], approvals: Approval[], spec: Spec): SiteEvent[] {
  const ev: SiteEvent[] = [];
  for (const c of commits) {
    const same = { en: c.subject, zh: c.subject };
    if (c.tags.length) ev.push({ at: c.at, type: 'release', title: same, hash: c.hash, tag: c.tags[0] });
    else ev.push({ at: c.at, type: 'commit', title: same, hash: c.hash });
  }
  for (const r of reports) ev.push({ at: `${r.date}T${r.time}:00.000Z`, type: 'report', title: r.title, ref: r.id });
  for (const a of approvals) if (a.requested) ev.push({ at: `${a.requested}T12:00:00.000Z`, type: 'decision', title: a.title, ref: a.id });
  for (const c of spec.changes) ev.push({ at: `${c.date}T12:00:00.000Z`, type: 'spec', title: { en: c.text, zh: c.text } });
  return ev.sort((a, b) => b.at.localeCompare(a.at));
}

export function loadSite(): Site {
  const spec = loadSpec();
  const approvals = loadApprovals();
  const { commits, head, latestTag } = loadGit();
  const reports = loadReports();
  const withTests = reports.find((r) => r.tests);
  return {
    spec,
    counts: count(spec.reqs),
    status: loadStatus(),
    approvals,
    pending: approvals.filter((a) => a.state === 'pending'),
    burndown: loadBurndown(),
    commits,
    head,
    latestTag,
    reports,
    latest: reports[0],
    tests: withTests?.tests ? { tests: withTests.tests, report: withTests } : undefined,
    events: buildEvents(commits, reports, approvals, spec),
    builtAt: new Date(),
  };
}
