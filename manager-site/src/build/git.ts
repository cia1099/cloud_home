import { REPO_ROOT, warn } from './util.ts';

export interface GitCommit {
  hash: string;
  /** UTC ISO timestamp */
  at: string;
  subject: string;
  tags: string[];
}

function git(args: string[]): string | null {
  const r = Bun.spawnSync(['git', ...args], { cwd: REPO_ROOT, stdout: 'pipe', stderr: 'pipe' });
  return r.exitCode === 0 ? r.stdout.toString().trim() : null;
}

const MAX_COMMITS = 400;

/** Hash, date, tags and subject only (no author data), newest first. */
export function loadGit(): { commits: GitCommit[]; head: string | null; latestTag: string | null } {
  const log = git(['log', `-n${MAX_COMMITS}`, '--format=%h%x1f%cI%x1f%D%x1f%s']);
  if (log === null) {
    warn('git history is not available; the timeline and the latest-build card are empty');
    return { commits: [], head: null, latestTag: null };
  }
  const commits = log
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      const [hash = '', iso = '', refs = '', subject = ''] = line.split('\x1f');
      return {
        hash,
        at: new Date(iso).toISOString(),
        subject,
        tags: refs.split(', ').filter((r) => r.startsWith('tag: ')).map((r) => r.slice(5)),
      };
    });
  return { commits, head: git(['rev-parse', '--short=7', 'HEAD']), latestTag: git(['describe', '--tags', '--abbrev=0', '--match', 'v*']) };
}
