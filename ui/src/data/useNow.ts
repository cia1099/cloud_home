import { useEffect, useState } from 'react'

/** The current time, refreshed every minute so day counts and expiry chips stay right on a long-open page. */
export function useNow(): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 60_000)
    return () => window.clearInterval(timer)
  }, [])
  return now
}
