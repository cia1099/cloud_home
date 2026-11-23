import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { Bi } from '../shared/i18n.ts';

export const REPO_ROOT = resolve(import.meta.dir, '../../..');
export const SITE_ROOT = resolve(import.meta.dir, '../..');
export const REPORTS_DIR = resolve(SITE_ROOT, process.env.REPORTS_DIR ?? '../reports');
export const OUT_DIR = resolve(SITE_ROOT, process.env.OUT_DIR ?? 'report_web');

export const warnings: string[] = [];
export const warn = (msg: string): void => {
  warnings.push(msg);
};

export function readText(path: string): string | null {
  return existsSync(path) ? readFileSync(path, 'utf8') : null;
}

export function readJson(path: string): unknown {
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (e) {
    throw new Error(`${path}: ${(e as Error).message}`);
  }
}

export const repoPath = (...p: string[]): string => resolve(REPO_ROOT, ...p);

export function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

export function asBi(v: unknown, where: string): Bi {
  if (isObject(v) && typeof v.en === 'string' && typeof v.zh === 'string') return { en: v.en, zh: v.zh };
  throw new Error(`${where}: expected {"en": "...", "zh": "..."}`);
}

export function asString(v: unknown, where: string): string {
  if (typeof v === 'string') return v;
  throw new Error(`${where}: expected a string`);
}

export function asArray(v: unknown, where: string): unknown[] {
  if (Array.isArray(v)) return v;
  throw new Error(`${where}: expected an array`);
}

export function asObject(v: unknown, where: string): Record<string, unknown> {
  if (isObject(v)) return v;
  throw new Error(`${where}: expected an object`);
}

export function asNumber(v: unknown, where: string): number {
  if (typeof v === 'number' && Number.isFinite(v)) return v;
  throw new Error(`${where}: expected a number`);
}

export function oneOf<T extends string>(v: unknown, allowed: readonly T[], where: string): T {
  if (typeof v === 'string' && (allowed as readonly string[]).includes(v)) return v as T;
  throw new Error(`${where}: expected one of ${allowed.join(', ')}`);
}

export const optBi = (v: unknown, where: string): Bi | undefined => (v === undefined ? undefined : asBi(v, where));
