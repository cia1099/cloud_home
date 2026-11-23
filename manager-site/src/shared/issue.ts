const ISSUE_URL = 'https://github.com/cia1099/cloud_home/issues/new';
export const COMMIT_URL = 'https://github.com/cia1099/cloud_home/commit/';

const clip = (s: string): string => (s.length > 80 ? `${s.slice(0, 79)}…` : s);

/** Pre-filled GitHub issue. Title prefixes are parsed by the manager, so they are never localized. */
function issue(title: string, body: string): string {
  return `${ISSUE_URL}?title=${encodeURIComponent(title)}&body=${encodeURIComponent(body)}`;
}

export const askUrl = (page: string): string =>
  issue('[Q] ', `**Question**\n\n\n\n---\nAbout page: ${page}`);

export const newRequirementUrl = (): string =>
  issue(
    '[ADD] ',
    '**New requirement**\n\nWhat do you want?\n\n\nPriority (P0 must / P1 should / P2 nice): P1\n\nHow will we know it works?\n\n',
  );

export const changeRequirementUrl = (id: string, title: string): string =>
  issue(`[CHG ${id}] ${clip(title)}`, `**Change request for ${id}: ${clip(title)}**\n\nWhat should change?\n\n\n`);

export const removeRequirementUrl = (id: string, title: string): string =>
  issue(`[DEL ${id}] ${clip(title)}`, `**Remove ${id}: ${clip(title)}**\n\nReason (optional):\n\n`);

export const decisionUrl = (id: string, title: string, verdict: 'approve' | 'deny'): string =>
  issue(`[DECISION ${id}] ${verdict}`, `**Decision ${id}: ${clip(title)}** — ${verdict}\n\nNote (optional):\n`);
