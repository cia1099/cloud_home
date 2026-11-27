import { useEffect, useRef, useState, type ReactNode } from 'react'
import { visibleRange } from '../files/model'

/** A scrolling list of fixed-height rows that only renders the rows in view (plus a few more). */
export function VirtualRows<T>({
  items,
  rowHeight,
  itemKey,
  render,
  label,
}: {
  items: readonly T[]
  rowHeight: number
  itemKey: (item: T) => string
  render: (item: T) => ReactNode
  label: string
}) {
  const scroller = useRef<HTMLDivElement>(null)
  const [metrics, setMetrics] = useState({ top: 0, height: 800 })

  useEffect(() => {
    const element = scroller.current
    if (!element) return
    const measure = () => setMetrics({ top: element.scrollTop, height: element.clientHeight })
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  const { first, last } = visibleRange(metrics.top, metrics.height, rowHeight, items.length)
  return (
    <div ref={scroller} className="scroller" onScroll={(event) => setMetrics({ top: event.currentTarget.scrollTop, height: event.currentTarget.clientHeight })}>
      <div className="virtual rows" role="list" aria-label={label} style={{ height: items.length * rowHeight }}>
        <div className="window" style={{ transform: `translateY(${first * rowHeight}px)` }}>
          {items.slice(first, last).map((item, i) => (
            <div key={itemKey(item)} role="listitem" className="rrow" style={{ height: rowHeight }} aria-posinset={first + i + 1} aria-setsize={items.length}>
              {render(item)}
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
