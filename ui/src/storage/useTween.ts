import { useEffect, useRef, useState } from 'react'

const DURATION_MS = 450

/** Eases from the shown value to `target` whenever it changes; with reduced motion it jumps. */
export function useTween(target: number): number {
  const [shown, setShown] = useState(target)
  const current = useRef(target)
  const [reduced] = useState(() => window.matchMedia('(prefers-reduced-motion: reduce)').matches)

  useEffect(() => {
    if (reduced || current.current === target) {
      current.current = target
      return
    }
    const from = current.current
    const start = performance.now()
    let frame = 0
    const step = (now: number) => {
      const progress = Math.min(1, (now - start) / DURATION_MS)
      const eased = 1 - (1 - progress) ** 3
      current.current = from + (target - from) * eased
      setShown(current.current)
      if (progress < 1) frame = requestAnimationFrame(step)
    }
    frame = requestAnimationFrame(step)
    return () => cancelAnimationFrame(frame)
  }, [target, reduced])

  return reduced ? target : shown
}
