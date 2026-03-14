import { useState, useMemo, useCallback, memo } from 'react'
import { cn } from '@/lib/utils'
import { ArrowUp, ArrowDown, ArrowUpDown } from 'lucide-react'
import { Button } from './ui/button'
import type { TrafficEntry } from '@/types/traffic'

type SortField = 'timestamp' | 'method' | 'status' | 'path' | 'duration' | 'size'
type SortDirection = 'asc' | 'desc'

interface TrafficListProps {
  traffic: TrafficEntry[]
  selectedId: string | null
  onSelect: (id: string) => void
}

interface ColumnConfig {
  field: SortField
  label: string
  width: string
  sortable: boolean
}

const columns: ColumnConfig[] = [
  { field: 'method', label: 'Method', width: 'w-16', sortable: true },
  { field: 'status', label: 'Status', width: 'w-14', sortable: true },
  { field: 'path', label: 'Path', width: 'flex-1', sortable: true },
  { field: 'duration', label: 'Time', width: 'w-16', sortable: true },
  { field: 'size', label: 'Size', width: 'w-16', sortable: true },
  { field: 'timestamp', label: 'Time', width: 'w-20', sortable: true },
]

export function TrafficList({ traffic, selectedId, onSelect }: TrafficListProps) {
  const [sortField, setSortField] = useState<SortField>('timestamp')
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc')

  const handleSort = useCallback((field: SortField) => {
    if (sortField === field) {
      setSortDirection(prev => prev === 'asc' ? 'desc' : 'asc')
    } else {
      setSortField(field)
      setSortDirection('desc')
    }
  }, [sortField])

  const sortedTraffic = useMemo(() => {
    return [...traffic].sort((a, b) => {
      let comparison = 0
      switch (sortField) {
        case 'timestamp':
          comparison = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
          break
        case 'method':
          comparison = a.request.method.localeCompare(b.request.method)
          break
        case 'status':
          comparison = (a.response?.status_code || 0) - (b.response?.status_code || 0)
          break
        case 'path':
          comparison = a.request.path.localeCompare(b.request.path)
          break
        case 'duration':
          comparison = (a.response?.duration_ms || 0) - (b.response?.duration_ms || 0)
          break
        case 'size':
          comparison = calculateSize(a) - calculateSize(b)
          break
      }
      return sortDirection === 'asc' ? comparison : -comparison
    })
  }, [traffic, sortField, sortDirection])

  if (traffic.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground p-4 text-center">
        <div>
          <p className="mb-2">No traffic captured yet</p>
          <p className="text-sm">
            Configure your browser or app to use localhost:8888 as proxy
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full" role="list" aria-label="Traffic entries">
      {/* Column Headers */}
      <div className="flex items-center gap-2 px-3 py-2 border-b bg-muted/30 text-xs font-medium text-muted-foreground">
        {columns.map((col) => (
          <ColumnHeader
            key={col.field}
            column={col}
            sortField={sortField}
            sortDirection={sortDirection}
            onSort={handleSort}
          />
        ))}
      </div>

      {/* Traffic List */}
      <div className="flex-1 overflow-auto">
        <div className="divide-y" role="listitem">
          {sortedTraffic.map((entry) => (
            <TrafficListItem
              key={entry.id}
              entry={entry}
              isSelected={entry.id === selectedId}
              onClick={() => onSelect(entry.id)}
            />
          ))}
        </div>
      </div>
    </div>
  )
}

interface ColumnHeaderProps {
  column: ColumnConfig
  sortField: SortField
  sortDirection: SortDirection
  onSort: (field: SortField) => void
}

const ColumnHeader = memo(function ColumnHeader({ column, sortField, sortDirection, onSort }: ColumnHeaderProps) {
  const isActive = sortField === column.field

  return (
    <Button
      variant="ghost"
      size="sm"
      className={cn(
        'h-6 px-1 justify-start font-medium hover:bg-muted',
        column.width
      )}
      onClick={() => column.sortable && onSort(column.field)}
      disabled={!column.sortable}
      aria-label={`Sort by ${column.label}`}
    >
      <span className="truncate">{column.label}</span>
      {column.sortable && (
        <span className="ml-1">
          {isActive ? (
            sortDirection === 'asc' ? (
              <ArrowUp className="h-3 w-3" />
            ) : (
              <ArrowDown className="h-3 w-3" />
            )
          ) : (
            <ArrowUpDown className="h-3 w-3 opacity-30" />
          )}
        </span>
      )}
    </Button>
  )
})

interface TrafficListItemProps {
  entry: TrafficEntry
  isSelected: boolean
  onClick: () => void
}

const TrafficListItem = memo(function TrafficListItem({ entry, isSelected, onClick }: TrafficListItemProps) {
  const methodClass = `method-${entry.request.method.toLowerCase()}`
  const statusClass = entry.response
    ? `status-${Math.floor(entry.response.status_code / 100)}xx`
    : ''

  const time = new Date(entry.timestamp).toLocaleTimeString()
  const size = entry.response
    ? formatSize(calculateSize(entry))
    : '-'

  return (
    <div
      className={cn(
        'px-3 py-2 cursor-pointer hover:bg-muted/50 transition-colors focus:outline-none focus:bg-muted/50',
        isSelected && 'bg-primary/10'
      )}
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onClick()
        }
      }}
      aria-selected={isSelected}
    >
      <div className="flex items-center gap-2 text-sm">
        <span className={cn('font-mono font-semibold w-16', methodClass)}>
          {entry.request.method}
        </span>
        <span className={cn('w-14 text-right', statusClass)}>
          {entry.response?.status_code || '-'}
        </span>
        <span className="flex-1 truncate font-mono text-xs" title={entry.request.path}>
          {entry.request.path}
        </span>
      </div>
      <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
        <span className="w-16" />
        <span className="w-14 text-right">{entry.response?.duration_ms || 0}ms</span>
        <span className="w-16">{size}</span>
        <span className="flex-1 truncate">{entry.request.host}</span>
        <span className="w-20 text-right">{time}</span>
      </div>
    </div>
  )
})

function calculateSize(entry: TrafficEntry): number {
  const reqSize = entry.request.body?.length || 0
  const resSize = entry.response?.body?.length || 0
  return reqSize + resSize
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`
}
