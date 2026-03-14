import { useState, useRef, useCallback, useEffect } from 'react'
import { cn } from '@/lib/utils'

interface ResizablePanelProps {
  children: React.ReactNode
  defaultWidth?: number
  minWidth?: number
  maxWidth?: number
  direction?: 'left' | 'right'
  className?: string
  storageKey?: string
  onResize?: (width: number) => void
}

export function ResizablePanel({
  children,
  defaultWidth = 400,
  minWidth = 200,
  maxWidth = 800,
  direction = 'right',
  className,
  storageKey,
  onResize,
}: ResizablePanelProps) {
  const [width, setWidth] = useState(() => {
    if (storageKey && typeof window !== 'undefined') {
      const saved = localStorage.getItem(storageKey)
      if (saved) {
        const parsed = parseInt(saved, 10)
        if (!isNaN(parsed) && parsed >= minWidth && parsed <= maxWidth) {
          return parsed
        }
      }
    }
    return defaultWidth
  })

  const [isResizing, setIsResizing] = useState(false)
  const panelRef = useRef<HTMLDivElement>(null)

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsResizing(true)
  }, [])

  useEffect(() => {
    if (!isResizing) return

    const handleMouseMove = (e: MouseEvent) => {
      if (!panelRef.current) return

      const containerRect = panelRef.current.parentElement?.getBoundingClientRect()
      if (!containerRect) return

      let newWidth: number
      if (direction === 'right') {
        newWidth = containerRect.right - e.clientX
      } else {
        newWidth = e.clientX - containerRect.left
      }

      newWidth = Math.min(maxWidth, Math.max(minWidth, newWidth))
      setWidth(newWidth)
      onResize?.(newWidth)

      if (storageKey) {
        localStorage.setItem(storageKey, newWidth.toString())
      }
    }

    const handleMouseUp = () => {
      setIsResizing(false)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }
  }, [isResizing, direction, minWidth, maxWidth, storageKey, onResize])

  return (
    <div
      ref={panelRef}
      className={cn('relative flex', className)}
      style={{ width: `${width}px` }}
    >
      {direction === 'right' && (
        <div
          className={cn(
            'absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 transition-colors z-10',
            isResizing && 'bg-primary'
          )}
          onMouseDown={handleMouseDown}
        />
      )}
      <div className="flex-1 overflow-hidden">{children}</div>
      {direction === 'left' && (
        <div
          className={cn(
            'absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 transition-colors z-10',
            isResizing && 'bg-primary'
          )}
          onMouseDown={handleMouseDown}
        />
      )}
    </div>
  )
}

interface ResizableSplitProps {
  left: React.ReactNode
  right: React.ReactNode
  defaultLeftWidth?: number
  minLeftWidth?: number
  maxLeftWidth?: number
  storageKey?: string
  className?: string
}

export function ResizableSplit({
  left,
  right,
  defaultLeftWidth = 50,
  minLeftWidth = 20,
  maxLeftWidth = 80,
  storageKey,
  className,
}: ResizableSplitProps) {
  const [leftPercent, setLeftPercent] = useState(() => {
    if (storageKey && typeof window !== 'undefined') {
      const saved = localStorage.getItem(storageKey)
      if (saved) {
        const parsed = parseFloat(saved)
        if (!isNaN(parsed) && parsed >= minLeftWidth && parsed <= maxLeftWidth) {
          return parsed
        }
      }
    }
    return defaultLeftWidth
  })

  const [isResizing, setIsResizing] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!isResizing) return

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return

      const rect = containerRef.current.getBoundingClientRect()
      const percent = ((e.clientX - rect.left) / rect.width) * 100
      const newPercent = Math.min(maxLeftWidth, Math.max(minLeftWidth, percent))

      setLeftPercent(newPercent)

      if (storageKey) {
        localStorage.setItem(storageKey, newPercent.toString())
      }
    }

    const handleMouseUp = () => {
      setIsResizing(false)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }
  }, [isResizing, minLeftWidth, maxLeftWidth, storageKey])

  return (
    <div ref={containerRef} className={cn('flex h-full', className)}>
      <div style={{ width: `${leftPercent}%` }} className="overflow-hidden">
        {left}
      </div>
      <div
        className={cn(
          'w-1 cursor-col-resize hover:bg-primary/50 transition-colors flex-shrink-0',
          isResizing && 'bg-primary'
        )}
        onMouseDown={(e) => {
          e.preventDefault()
          setIsResizing(true)
        }}
      />
      <div style={{ width: `${100 - leftPercent}%` }} className="overflow-hidden">
        {right}
      </div>
    </div>
  )
}
