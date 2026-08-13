import { useEffect, useRef, useState, useCallback } from 'react'
import { RotateCcw, Theater, Copy, Download } from 'lucide-react'
import { cn } from '@/lib/utils'

interface TrafficContextMenuProps {
  /** The traffic entry ID this menu is acting on. */
  entryId: string | null
  /** The screen coordinates where the menu should appear. */
  position: { x: number; y: number } | null
  /** Called when the menu requests to save the entry to replay. */
  onSaveToReplay: (id: string) => void
  /** Called when the menu requests to create a mock from the entry. */
  onCreateMock: (id: string) => void
  /** Called when the menu requests to copy the entry as cURL. */
  onCopyAsCurl: (id: string) => void
  /** Called when the menu requests to export the entry as HAR. */
  onExportHar: (id: string) => void
  /** Called when the menu is dismissed. */
  onClose: () => void
}

/**
 * Lightweight right-click context menu for traffic list rows.
 * Renders a floating menu at the cursor position and closes on
 * outside click / Escape / scroll.
 */
export function TrafficContextMenu({
  entryId,
  position,
  onSaveToReplay,
  onCreateMock,
  onCopyAsCurl,
  onExportHar,
  onClose,
}: TrafficContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null)
  const [adjustedPos, setAdjustedPos] = useState<{
    x: number
    y: number
  } | null>(null)

  // Adjust position so the menu doesn't overflow the viewport
  useEffect(() => {
    if (!position) {
      setAdjustedPos(null)
      return
    }
    const menu = menuRef.current
    const w = menu?.offsetWidth ?? 200
    const h = menu?.offsetHeight ?? 160
    const x = Math.min(position.x, window.innerWidth - w - 8)
    const y = Math.min(position.y, window.innerHeight - h - 8)
    setAdjustedPos({ x: Math.max(8, x), y: Math.max(8, y) })
  }, [position])

  // Close on outside click / Escape / scroll
  useEffect(() => {
    if (!entryId) return
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    const handleScroll = () => onClose()
    document.addEventListener('mousedown', handleClick)
    document.addEventListener('keydown', handleKey)
    window.addEventListener('scroll', handleScroll, true)
    return () => {
      document.removeEventListener('mousedown', handleClick)
      document.removeEventListener('keydown', handleKey)
      window.removeEventListener('scroll', handleScroll, true)
    }
  }, [entryId, onClose])

  const run = useCallback(
    (fn: (id: string) => void) => {
      if (entryId) fn(entryId)
      onClose()
    },
    [entryId, onClose],
  )

  if (!entryId || !adjustedPos) return null

  const items = [
    {
      icon: RotateCcw,
      label: 'Save to Replay',
      onClick: () => run(onSaveToReplay),
    },
    {
      icon: Theater,
      label: 'Create Mock',
      onClick: () => run(onCreateMock),
    },
    {
      icon: Copy,
      label: 'Copy as cURL',
      onClick: () => run(onCopyAsCurl),
    },
    {
      icon: Download,
      label: 'Export as HAR',
      onClick: () => run(onExportHar),
    },
  ]

  return (
    <div
      ref={menuRef}
      className={cn(
        'fixed z-[70] min-w-[180px] rounded-md border border-border bg-popover p-1 shadow-md',
        'animate-in fade-in zoom-in-95 duration-100',
      )}
      style={{ left: adjustedPos.x, top: adjustedPos.y }}
    >
      {items.map((item) => (
        <button
          key={item.label}
          onClick={item.onClick}
          className={cn(
            'flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm',
            'hover:bg-accent hover:text-accent-foreground',
          )}
        >
          <item.icon className="h-3.5 w-3.5" />
          {item.label}
        </button>
      ))}
    </div>
  )
}
