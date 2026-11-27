/** 1 s, 2 s, 4 s ... capped at 10 s (design/app-spec.md section 9). */
export function backoffDelay(attempt: number): number {
  return Math.min(1000 * 2 ** attempt, 10_000)
}
