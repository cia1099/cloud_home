import { useEffect, useRef, useState, type DragEvent } from 'react'

const carriesFiles = (event: DragEvent) => event.dataTransfer.types.includes('Files')

/**
 * Drag-and-drop of files over an element. Enter/leave events fire for every child the cursor
 * crosses, so they are counted; state only changes at the edges, which keeps the page under the
 * cursor from re-rendering while dragging. `onDrop` runs inside the drop event (the entries of the
 * list are only readable there).
 */
export function useFileDrop(enabled: boolean, onDrop: (items: DataTransferItemList) => void) {
  const [dragging, setDragging] = useState(false)
  const depth = useRef(0)

  const end = () => {
    depth.current = 0
    setDragging(false)
  }

  // Esc ends a drag without a drop; the browser reports it as dragend on the source or leave on the window.
  useEffect(() => {
    window.addEventListener('dragend', end)
    window.addEventListener('drop', end)
    return () => {
      window.removeEventListener('dragend', end)
      window.removeEventListener('drop', end)
    }
  }, [])

  return {
    dragging: dragging && enabled,
    handlers: {
      onDragEnter: (event: DragEvent) => {
        if (!carriesFiles(event)) return
        event.preventDefault()
        depth.current += 1
        setDragging(true)
      },
      onDragOver: (event: DragEvent) => {
        if (!carriesFiles(event)) return
        event.preventDefault()
        event.dataTransfer.dropEffect = enabled ? 'copy' : 'none'
      },
      onDragLeave: (event: DragEvent) => {
        if (!carriesFiles(event)) return
        depth.current -= 1
        if (depth.current <= 0) end()
      },
      onDrop: (event: DragEvent) => {
        if (!carriesFiles(event)) return
        event.preventDefault()
        end()
        if (enabled) onDrop(event.dataTransfer.items)
      },
    },
  }
}
