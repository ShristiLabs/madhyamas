import { useState, useMemo, useCallback, useEffect, useRef } from 'react'
import { useTraffic, useTrafficCount, useClearTraffic } from '@/hooks/useTraffic'
import { TrafficList } from './TrafficList'
import { TrafficDetail } from './TrafficDetail'
import { TrafficToolbar } from './TrafficToolbar'
import { ToolsSidebar } from './ToolsSidebar'
import { Button } from './ui/button'
import { Trash2, RefreshCw, Wrench, Keyboard } from 'lucide-react'
import type { TrafficEntry, TrafficFilter } from '@/types/traffic'
import { cn } from '@/lib/utils'

const STORAGE_KEY_LIST_WIDTH = 'proxyforge-traffic-list-width'
const STORAGE_KEY_TOOLS_WIDTH = 'proxyforge-tools-width'
const DEFAULT_LIST_WIDTH = 40
const DEFAULT_TOOLS_WIDTH = 400
const MIN_LIST_WIDTH = 20
const MAX_LIST_WIDTH = 60
const MIN_TOOLS_WIDTH = 300
const MAX_TOOLS_WIDTH = 600

export function TrafficView() {
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [filter, setFilter] = useState<TrafficFilter>({})
  const [showTools, setShowTools] = useState(false)
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [listWidth, setListWidth] = useState(() => {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem(STORAGE_KEY_LIST_WIDTH)
      return saved ? parseInt(saved, 10) : DEFAULT_LIST_WIDTH
    }
    return DEFAULT_LIST_WIDTH
  })
  const [toolsWidth, setToolsWidth] = useState(() => {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem(STORAGE_KEY_TOOLS_WIDTH)
      return saved ? parseInt(saved, 10) : DEFAULT_TOOLS_WIDTH
    }
    return DEFAULT_TOOLS_WIDTH
  })

  const { data: traffic, isLoading, refetch } = useTraffic(filter)
  const { data: count } = useTrafficCount()
  const clearTraffic = useClearTraffic()

  const containerRef = useRef<HTMLDivElement>(null)
  const [isResizingList, setIsResizingList] = useState(false)
  const [isResizingTools, setIsResizingTools] = useState(false)

  const selectedEntry = useMemo(() => {
    if (!traffic || !selectedId) return null
    return traffic.find((t: TrafficEntry) => t.id === selectedId) || null
  }, [traffic, selectedId])

  const handleClear = useCallback(() => {
    if (confirm('Clear all traffic?')) {
      clearTraffic.mutate()
      setSelectedId(null)
    }
  }, [clearTraffic])

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger shortcuts when typing in inputs
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return
      }

      switch (e.key.toLowerCase()) {
        case 'r':
          if (!e.metaKey && !e.ctrlKey) {
            e.preventDefault()
            refetch()
          }
          break
        case 'c':
          if (!e.metaKey && !e.ctrlKey) {
            e.preventDefault()
            handleClear()
          }
          break
        case 't':
          e.preventDefault()
          setShowTools(prev => !prev)
          break
        case '?':
          e.preventDefault()
          setShowShortcuts(prev => !prev)
          break
        case 'escape':
          setShowShortcuts(false)
          break
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [refetch, handleClear])

  // List panel resize handlers
  const handleListMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsResizingList(true)
  }, [])

  const handleToolsMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsResizingTools(true)
  }, [])

  useEffect(() => {
    if (!isResizingList && !isResizingTools) return

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return
      const rect = containerRef.current.getBoundingClientRect()

      if (isResizingList) {
        const percent = ((e.clientX - rect.left) / rect.width) * 100
        const newWidth = Math.min(MAX_LIST_WIDTH, Math.max(MIN_LIST_WIDTH, percent))
        setListWidth(newWidth)
        localStorage.setItem(STORAGE_KEY_LIST_WIDTH, newWidth.toString())
      } else if (isResizingTools && showTools) {
        const percent = ((rect.right - e.clientX) / rect.width) * 100
        const newWidth = Math.min(MAX_TOOLS_WIDTH, Math.max(MIN_TOOLS_WIDTH, rect.width * (percent / 100)))
        setToolsWidth(newWidth)
        localStorage.setItem(STORAGE_KEY_TOOLS_WIDTH, newWidth.toString())
      }
    }

    const handleMouseUp = () => {
      setIsResizingList(false)
      setIsResizingTools(false)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }
  }, [isResizingList, isResizingTools, showTools])

  return (
    <div className="h-full flex flex-col">
      <TrafficToolbar
        filter={filter}
        onFilterChange={setFilter}
        count={count || 0}
      />

      <div className="flex-1 flex overflow-hidden border-t" ref={containerRef}>
        <div className="flex-1 flex flex-col min-w-0">
          <div className="px-3 py-2 border-b bg-muted/50 flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {count ?? 0} requests
            </span>
            <div className="flex gap-2">
              <Button
                variant={showTools ? 'default' : 'ghost'}
                size="sm"
                onClick={() => setShowTools(!showTools)}
                title="Toggle Tools Panel (T)"
              >
                <Wrench className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowShortcuts(!showShortcuts)}
                title="Keyboard Shortcuts (?)"
              >
                <Keyboard className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => refetch()}
                title="Refresh (R)"
              >
                <RefreshCw className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleClear}
                disabled={clearTraffic.isPending}
                title="Clear All (C)"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          </div>

          <div className="flex-1 flex overflow-hidden">
            {/* Traffic List Panel */}
            <div
              style={{ width: `${listWidth}%` }}
              className="flex flex-col min-w-[200px]"
            >
              <div className="flex-1 overflow-auto">
                {isLoading ? (
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    Loading...
                  </div>
                ) : (
                  <TrafficList
                    traffic={traffic || []}
                    selectedId={selectedId}
                    onSelect={setSelectedId}
                  />
                )}
              </div>
            </div>

            {/* Resize Handle for List */}
            <div
              className={cn(
                'w-1 cursor-col-resize hover:bg-primary/50 transition-colors flex-shrink-0',
                isResizingList && 'bg-primary'
              )}
              onMouseDown={handleListMouseDown}
            />

            {/* Traffic Detail Panel */}
            <div className="flex-1 flex flex-col min-w-0">
              {selectedEntry ? (
                <TrafficDetail entry={selectedEntry} />
              ) : (
                <div className="flex-1 flex items-center justify-center text-muted-foreground">
                  Select a request to view details
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Tools Sidebar with Resize */}
        {showTools && (
          <>
            {/* Resize Handle for Tools */}
            <div
              className={cn(
                'w-1 cursor-col-resize hover:bg-primary/50 transition-colors flex-shrink-0',
                isResizingTools && 'bg-primary'
              )}
              onMouseDown={handleToolsMouseDown}
            />
            <div
              style={{ width: `${toolsWidth}px` }}
              className="border-l flex-shrink-0"
            >
              <ToolsSidebar
                selectedEntry={selectedEntry}
                isOpen={showTools}
                onClose={() => setShowTools(false)}
              />
            </div>
          </>
        )}
      </div>

      {/* Keyboard Shortcuts Modal */}
      {showShortcuts && (
        <div
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onClick={() => setShowShortcuts(false)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md w-full mx-4"
            onClick={e => e.stopPropagation()}
          >
            <h2 className="text-lg font-semibold mb-4">Keyboard Shortcuts</h2>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Refresh traffic</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">R</kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Clear all traffic</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">C</kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Toggle tools panel</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">T</kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Show shortcuts</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">?</kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Close modal</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">Esc</kbd>
              </div>
            </div>
            <Button
              className="mt-4 w-full"
              onClick={() => setShowShortcuts(false)}
            >
              Close
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
