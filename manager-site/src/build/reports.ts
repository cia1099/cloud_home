import { existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import type { Bi } from '../shared/i18n.ts';
import { isIcon, type IconName } from '../shared/icons.ts';
import { asArray, asBi, asNumber, asObject, asString, oneOf, readJson, REPORTS_DIR } from './util.ts';

export interface Section {
  id: string;
  component: string;
  title?: Bi;
  intro?: Bi;
  caption?: Bi;
  data: Record<string, unknown>;
}

export interface Suite {
  name: string;
  passed: number;
  failed: number;
  skipped: number;
}

export interface Tests {
  suites: Suite[];
}

export interface Report {
  id: string;
  /** YYYY-MM-DD and HH:MM (UTC), both taken from the folder name */
  date: string;
  time: string;
  slug: string;
  kind: 'weekly' | 'milestone';
  title: Bi;
  summary: Bi;
  chip?: { tone: 'ok' | 'accent' | 'warn' | 'neutral'; icon: IconName; text: Bi };
  cover: { style: 'c1' | 'c2' | 'c3' | 'c-ms'; icon: IconName };
  /** file name (relative to the report folder) of a bespoke page module, if the report ships its own design */
  page?: string;
  pageCss?: string;
  sections: Section[];
  tests?: Tests;
}

const ID = /^(\d{4}-\d{2}-\d{2})-(\d{2})(\d{2})-([a-z0-9][a-z0-9-]*)$/;

function parseTests(path: string): Tests {
  const o = asObject(readJson(path), path);
  return {
    suites: asArray(o.suites, `${path}: suites`).map((s, i) => {
      const w = `${path}: suites[${i}]`;
      const x = asObject(s, w);
      return { name: asString(x.name, `${w}.name`), passed: asNumber(x.passed, `${w}.passed`), failed: asNumber(x.failed, `${w}.failed`), skipped: asNumber(x.skipped, `${w}.skipped`) };
    }),
  };
}

export function loadReports(dir = REPORTS_DIR): Report[] {
  if (!existsSync(dir)) return [];
  const reports: Report[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const m = ID.exec(entry.name);
    if (!m) throw new Error(`reports/${entry.name}: folder names look like 2026-11-24-1500-some-slug (date, HHMM in UTC, slug)`);
    const where = `reports/${entry.name}/report.json`;
    const o = asObject(readJson(join(dir, entry.name, 'report.json')), where);
    const chip = o.chip === undefined ? undefined : asObject(o.chip, `${where}: chip`);
    const cover = asObject(o.cover ?? { style: 'c1', icon: 'reports' }, `${where}: cover`);
    const icon = (v: unknown, w: string): IconName => {
      const s = asString(v, w);
      if (!isIcon(s)) throw new Error(`${w}: unknown icon "${s}"`);
      return s;
    };
    const page = o.page === undefined ? undefined : asString(o.page, `${where}: page`);
    if (page && !existsSync(join(dir, entry.name, page))) throw new Error(`${where}: page "${page}" does not exist`);
    const pageCss = o.page_css === undefined ? undefined : asString(o.page_css, `${where}: page_css`);
    const sections = asArray(o.sections ?? [], `${where}: sections`).map((s, i) => {
      const x = asObject(s, `${where}: sections[${i}]`);
      return {
        id: asString(x.id, `${where}: sections[${i}].id`),
        component: asString(x.component, `${where}: sections[${i}].component`),
        title: x.title === undefined ? undefined : asBi(x.title, `${where}: sections[${i}].title`),
        intro: x.intro === undefined ? undefined : asBi(x.intro, `${where}: sections[${i}].intro`),
        caption: x.caption === undefined ? undefined : asBi(x.caption, `${where}: sections[${i}].caption`),
        data: asObject(x.data ?? {}, `${where}: sections[${i}].data`),
      };
    });
    if (!page && !sections.length) throw new Error(`${where}: a report needs "sections" or a bespoke "page"`);
    const testsPath = join(dir, entry.name, 'tests.json');
    reports.push({
      id: entry.name,
      date: m[1] ?? '',
      time: `${m[2]}:${m[3]}`,
      slug: m[4] ?? '',
      kind: oneOf(o.kind, ['weekly', 'milestone'] as const, `${where}: kind`),
      title: asBi(o.title, `${where}: title`),
      summary: asBi(o.summary, `${where}: summary`),
      chip: chip && { tone: oneOf(chip.tone, ['ok', 'accent', 'warn', 'neutral'] as const, `${where}: chip.tone`), icon: icon(chip.icon ?? 'check', `${where}: chip.icon`), text: asBi(chip.text, `${where}: chip.text`) },
      cover: { style: oneOf(cover.style, ['c1', 'c2', 'c3', 'c-ms'] as const, `${where}: cover.style`), icon: icon(cover.icon, `${where}: cover.icon`) },
      page,
      pageCss,
      sections,
      tests: existsSync(testsPath) ? parseTests(testsPath) : undefined,
    });
  }
  return reports.sort((a, b) => b.id.localeCompare(a.id));
}
