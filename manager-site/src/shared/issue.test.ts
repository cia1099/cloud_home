import { expect, test } from 'bun:test';
import { acceptRequirementUrl, askUrl, changeRequirementUrl, decisionUrl, newRequirementUrl, removeRequirementUrl } from './issue.ts';

function decode(url: string): { title: string; body: string } {
  const u = new URL(url);
  expect(u.origin + u.pathname).toBe('https://github.com/cia1099/cloud_home/issues/new');
  return { title: u.searchParams.get('title') ?? '', body: u.searchParams.get('body') ?? '' };
}

test('ask a question', () => {
  const { title, body } = decode(askUrl('/requirements/'));
  expect(title).toBe('[Q] ');
  expect(body).toContain('About page: /requirements/');
});

test('new requirement', () => {
  const { title, body } = decode(newRequirementUrl());
  expect(title).toBe('[ADD] ');
  expect(body).toContain('What do you want?');
});

test('change and remove carry the id and the visible title', () => {
  const t = 'Account settings & "sign-in" #1';
  const c = decode(changeRequirementUrl('R-012', t));
  expect(c.title).toBe(`[CHG R-012] ${t}`);
  expect(c.body).toContain('What should change?');
  const d = decode(removeRequirementUrl('R-012', t));
  expect(d.title).toBe(`[DEL R-012] ${t}`);
  expect(d.body).toContain('R-012');
});

test('accept carries the id, the visible title and an optional note', () => {
  const a = decode(acceptRequirementUrl('R-012', 'Upload & "share"'));
  expect(a.title).toBe('[OK R-012] Upload & "share"');
  expect(a.body).toContain('Note (optional)');
});

test('decisions', () => {
  const pick = decode(decisionUrl('A-003', 'Cloud icon', 'choose B'));
  expect(pick.title).toBe('[DECISION A-003] choose B');
  expect(pick.body).toContain('A-003: Cloud icon');
  expect(decode(decisionUrl('A-003', 'Tag v0.3.1', 'approve')).title).toBe('[DECISION A-003] approve');
  const deny = decode(decisionUrl('A-003', 'Tag v0.3.1', 'deny'));
  expect(deny.title).toBe('[DECISION A-003] deny');
  expect(deny.body).toContain('A-003: Tag v0.3.1');
});

test('URLs stay short and Chinese is encoded', () => {
  const u = changeRequirementUrl('R-001', '经理报告网站 '.repeat(20));
  expect(u.length).toBeLessThan(2000);
  expect(u).not.toMatch(/[^\x20-\x7e]/);
});
