import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react'
import { Icon, type IconName } from './Icon'

export interface MenuItem {
  label: string
  icon?: IconName
  onSelect: () => void
  danger?: boolean
  /** Shows a check mark (sort menu, view menu). */
  checked?: boolean
  /** Draws a separator above this item. */
  separated?: boolean
}

export interface MenuAnchor {
  x: number
  y: number
}

/** Popover menu at a fixed viewport position: arrows, Home/End, Esc, click outside, focus returns to the opener. */
export function Menu({ anchor, items, label, onClose }: { anchor: MenuAnchor; items: MenuItem[]; label: string; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null)
  const [position, setPosition] = useState<MenuAnchor>(anchor)

  useLayoutEffect(() => {
    const box = ref.current?.getBoundingClientRect()
    if (!box) return
    // Keep the menu inside the viewport.
    setPosition({
      x: Math.max(8, Math.min(anchor.x, window.innerWidth - box.width - 8)),
      y: Math.max(8, Math.min(anchor.y, window.innerHeight - box.height - 8)),
    })
  }, [anchor])

  useEffect(() => {
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null
    ref.current?.querySelector<HTMLElement>('button')?.focus()
    const outside = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose()
    }
    document.addEventListener('pointerdown', outside)
    return () => {
      document.removeEventListener('pointerdown', outside)
      opener?.focus()
    }
  }, [onClose])

  const onKeyDown = (event: KeyboardEvent) => {
    const buttons = [...(ref.current?.querySelectorAll<HTMLElement>('button') ?? [])]
    const at = buttons.indexOf(document.activeElement as HTMLElement)
    const move = (index: number) => {
      event.preventDefault()
      buttons[(index + buttons.length) % buttons.length]?.focus()
    }
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      onClose()
    } else if (event.key === 'ArrowDown') move(at + 1)
    else if (event.key === 'ArrowUp') move(at - 1)
    else if (event.key === 'Home') move(0)
    else if (event.key === 'End') move(buttons.length - 1)
    else if (event.key === 'Tab') onClose()
  }

  return (
    <div ref={ref} className="menu" role="menu" aria-label={label} style={{ left: position.x, top: position.y }} onKeyDown={onKeyDown}>
      {items.map((item) => (
        <div key={item.label} role="none">
          {item.separated && <hr />}
          <button
            type="button"
            role={item.checked === undefined ? 'menuitem' : 'menuitemradio'}
            aria-checked={item.checked}
            className={item.danger ? 'dng' : undefined}
            onClick={() => {
              onClose()
              item.onSelect()
            }}
          >
            {item.icon ? <Icon name={item.icon} size={18} /> : <span className="menu-gap" />}
            <span className="menu-label">{item.label}</span>
            {item.checked && <Icon name="check" size={16} className="menu-check" />}
          </button>
        </div>
      ))}
    </div>
  )
}
