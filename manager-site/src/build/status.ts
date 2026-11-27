import { ROLES, type Bi, type Role } from '../shared/i18n.ts';
import { asArray, asBi, asObject, asString, oneOf, optBi, readText, repoPath } from './util.ts';

export interface RoleState {
  working: boolean;
  task: Bi;
  at?: string;
}

export interface Check {
  name: Bi;
  state: 'ok' | 'warn' | 'neutral';
  detail: Bi;
}

export interface SiteStatus {
  updated?: string;
  chip?: { tone: 'ok' | 'accent' | 'warn'; text: Bi };
  headline?: Bi;
  lede?: Bi;
  /** capabilities that existed before the plan began, per area key (shown as "N built before the plan") */
  builtBeforePlan: Record<string, number>;
  roles: Partial<Record<Role, RoleState>>;
  checks: Check[];
}

const FENCE = /```json site-status\n([\s\S]*?)\n```/;

/** The machine-readable block inside plan/STATUS.md (format documented in manager-site/README.md). */
export function parseStatus(text: string | null): SiteStatus {
  const empty: SiteStatus = { builtBeforePlan: {}, roles: {}, checks: [] };
  const raw = text && FENCE.exec(text)?.[1];
  if (!raw) return empty;
  let j: Record<string, unknown>;
  try {
    j = asObject(JSON.parse(raw), 'plan/STATUS.md site-status');
  } catch (e) {
    throw new Error(`plan/STATUS.md: the "json site-status" block is not valid JSON (${(e as Error).message})`);
  }
  const roles: SiteStatus['roles'] = {};
  for (const [k, v] of Object.entries(asObject(j.roles ?? {}, 'site-status.roles'))) {
    const r = oneOf(k, ROLES, `site-status.roles.${k}`);
    const o = asObject(v, `site-status.roles.${k}`);
    roles[r] = { working: o.working === true, task: asBi(o.task, `site-status.roles.${k}.task`), at: o.at === undefined ? undefined : asString(o.at, `site-status.roles.${k}.at`) };
  }
  const built: Record<string, number> = {};
  for (const [k, v] of Object.entries(asObject(j.built_before_plan ?? {}, 'site-status.built_before_plan'))) built[k] = Number(v);
  const chip = j.chip === undefined ? undefined : asObject(j.chip, 'site-status.chip');
  return {
    updated: j.updated === undefined ? undefined : asString(j.updated, 'site-status.updated'),
    chip: chip && { tone: oneOf(chip.tone, ['ok', 'accent', 'warn'] as const, 'site-status.chip.tone'), text: asBi(chip.text, 'site-status.chip.text') },
    headline: optBi(j.headline, 'site-status.headline'),
    lede: optBi(j.lede, 'site-status.lede'),
    builtBeforePlan: built,
    roles,
    checks: asArray(j.checks ?? [], 'site-status.checks').map((c, i) => {
      const o = asObject(c, `site-status.checks[${i}]`);
      return { name: asBi(o.name, `checks[${i}].name`), state: oneOf(o.state, ['ok', 'warn', 'neutral'] as const, `checks[${i}].state`), detail: asBi(o.detail, `checks[${i}].detail`) };
    }),
  };
}

export const loadStatus = (): SiteStatus => parseStatus(readText(repoPath('plan/STATUS.md')));
