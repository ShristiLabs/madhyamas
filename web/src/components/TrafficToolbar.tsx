import { useState, useCallback } from 'react'
import { Search, Filter, X, AlertCircle, Clock, Globe, Zap } from 'lucide-react'
import { Input } from './ui/input'
import { Button } from './ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from './ui/select'
import type { TrafficFilter, HttpMethod } from '@/types/traffic'

interface TrafficToolbarProps {
  filter: TrafficFilter
  onFilterChange: (filter: TrafficFilter) => void
  count?: number
}

const METHODS = ['ALL', 'GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const
const STATUS_CODES = [
  { value: 'ALL', label: 'All Status' },
  { value: '2xx', label: '2xx Success' },
  { value: '3xx', label: '3xx Redirect' },
  { value: '4xx', label: '4xx Client Error' },
  { value: '5xx', label: '5xx Server Error' },
] as const

interface QuickFilter {
  id: string
  label: string
  icon: React.ComponentType<{ className?: string }>
  filter: TrafficFilter
  description: string
}

const QUICK_FILTERS: QuickFilter[] = [
  {
    id: 'errors',
    label: 'Errors',
    icon: AlertCircle,
    filter: { statusCode: '4xx' },
    description: '4xx and 5xx responses',
  },
  {
    id: 'slow',
    label: 'Slow',
    icon: Clock,
    filter: { minDuration: 1000 },
    description: 'Requests > 1s',
  },
  {
    id: 'api',
    label: 'API',
    icon: Globe,
    filter: { url: '/api/' },
    description: 'API endpoints',
  },
]

export function TrafficToolbar({ filter, onFilterChange, count }: TrafficToolbarProps) {
  const [showAdvanced, setShowAdvanced] = useState(false)

  const handleSearchChange = useCallback((search: string) => {
    onFilterChange({ ...filter, search: search || undefined })
  }, [filter, onFilterChange])

  const handleMethodChange = useCallback((method: string) => {
    onFilterChange({
      ...filter,
      method: method === 'ALL' ? undefined : method as HttpMethod
    })
  }, [filter, onFilterChange])

  const handleStatusChange = useCallback((status: string) => {
    onFilterChange({
      ...filter,
      statusCode: status === 'ALL' ? undefined : status
    })
  }, [filter, onFilterChange])

  const handleUrlChange = useCallback((url: string) => {
    onFilterChange({ ...filter, url: url || undefined })
  }, [filter, onFilterChange])

  const handleQuickFilter = useCallback((quickFilter: QuickFilter) => {
    // Toggle the quick filter
    const isActive = isQuickFilterActive(filter, quickFilter)
    if (isActive) {
      // Remove the quick filter
      const newFilter: TrafficFilter = { ...filter }
      if (quickFilter.filter.statusCode) delete newFilter.statusCode
      if (quickFilter.filter.minDuration) delete newFilter.minDuration
      if (quickFilter.filter.url) delete newFilter.url
      onFilterChange(newFilter)
    } else {
      onFilterChange({ ...filter, ...quickFilter.filter })
    }
  }, [filter, onFilterChange])

  const handleClearFilters = useCallback(() => {
    onFilterChange({})
  }, [onFilterChange])

  const hasActiveFilters = Boolean(
    filter.search || filter.method || filter.statusCode || filter.url || filter.minDuration
  )

  const activeFilterCount = [
    filter.search,
    filter.method,
    filter.statusCode,
    filter.url,
    filter.minDuration,
  ].filter(Boolean).length

  return (
    <div className="border-b">
      <div className="px-4 py-3 flex items-center gap-3 flex-wrap">
        {/* Search */}
        <div className="relative flex-1 max-w-md min-w-[200px]">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search traffic... (supports regex)"
            className="pl-9"
            value={filter.search || ''}
            onChange={(e) => handleSearchChange(e.target.value)}
            aria-label="Search traffic"
          />
        </div>

        {/* Quick Filters */}
        <div className="flex items-center gap-1">
          {QUICK_FILTERS.map((qf) => {
            const isActive = isQuickFilterActive(filter, qf)
            const Icon = qf.icon
            return (
              <Button
                key={qf.id}
                variant={isActive ? 'default' : 'outline'}
                size="sm"
                onClick={() => handleQuickFilter(qf)}
                className="h-8"
                title={qf.description}
              >
                <Icon className="h-4 w-4 mr-1" />
                {qf.label}
              </Button>
            )
          })}
        </div>

        {/* Standard Filters */}
        <div className="flex items-center gap-2">
          <Filter className="h-4 w-4 text-muted-foreground" />
          <Select value={filter.method || 'ALL'} onValueChange={handleMethodChange}>
            <SelectTrigger className="w-28" aria-label="Filter by method">
              <SelectValue placeholder="Method" />
            </SelectTrigger>
            <SelectContent>
              {METHODS.map((method) => (
                <SelectItem key={method} value={method}>
                  {method === 'ALL' ? 'All Methods' : method}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Select value={filter.statusCode || 'ALL'} onValueChange={handleStatusChange}>
            <SelectTrigger className="w-32" aria-label="Filter by status">
              <SelectValue placeholder="Status" />
            </SelectTrigger>
            <SelectContent>
              {STATUS_CODES.map((status) => (
                <SelectItem key={status.value} value={status.value}>
                  {status.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* URL Filter */}
        <Input
          placeholder="Filter by URL..."
          className="w-48"
          value={filter.url || ''}
          onChange={(e) => handleUrlChange(e.target.value)}
          aria-label="Filter by URL"
        />

        {/* Advanced Filters Toggle */}
        <Button
          variant={showAdvanced ? 'default' : 'ghost'}
          size="sm"
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="h-8"
        >
          <Zap className="h-4 w-4 mr-1" />
          Advanced
        </Button>

        {/* Clear Filters */}
        {hasActiveFilters && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClearFilters}
            className="h-8 text-muted-foreground"
          >
            <X className="h-4 w-4 mr-1" />
            Clear
            {activeFilterCount > 0 && (
              <span className="ml-1 rounded-full bg-muted px-1.5 text-xs">
                {activeFilterCount}
              </span>
            )}
          </Button>
        )}

        {/* Count */}
        {count !== undefined && (
          <span className="text-sm text-muted-foreground ml-auto">
            {count} requests
          </span>
        )}
      </div>

      {/* Advanced Filters Row */}
      {showAdvanced && (
        <div className="px-4 py-2 bg-muted/30 border-t flex items-center gap-4 flex-wrap">
          <div className="flex items-center gap-2">
            <label className="text-sm text-muted-foreground">Min Duration (ms):</label>
            <Input
              type="number"
              className="w-24 h-8"
              placeholder="0"
              value={filter.minDuration || ''}
              onChange={(e) => onFilterChange({
                ...filter,
                minDuration: e.target.value ? parseInt(e.target.value, 10) : undefined
              })}
              aria-label="Minimum duration in milliseconds"
            />
          </div>

          <div className="flex items-center gap-2">
            <label className="text-sm text-muted-foreground">Max Duration (ms):</label>
            <Input
              type="number"
              className="w-24 h-8"
              placeholder="∞"
              value={filter.maxDuration || ''}
              onChange={(e) => onFilterChange({
                ...filter,
                maxDuration: e.target.value ? parseInt(e.target.value, 10) : undefined
              })}
              aria-label="Maximum duration in milliseconds"
            />
          </div>

          <div className="flex items-center gap-2">
            <label className="text-sm text-muted-foreground">Host:</label>
            <Input
              className="w-40 h-8"
              placeholder="example.com"
              value={filter.host || ''}
              onChange={(e) => onFilterChange({
                ...filter,
                host: e.target.value || undefined
              })}
              aria-label="Filter by host"
            />
          </div>

          <div className="flex items-center gap-2">
            <label className="text-sm text-muted-foreground">Content-Type:</label>
            <Select
              value={filter.contentType || 'ALL'}
              onValueChange={(val) => onFilterChange({
                ...filter,
                contentType: val === 'ALL' ? undefined : val
              })}
            >
              <SelectTrigger className="w-32 h-8" aria-label="Filter by content type">
                <SelectValue placeholder="All Types" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="ALL">All Types</SelectItem>
                <SelectItem value="json">JSON</SelectItem>
                <SelectItem value="html">HTML</SelectItem>
                <SelectItem value="xml">XML</SelectItem>
                <SelectItem value="javascript">JavaScript</SelectItem>
                <SelectItem value="css">CSS</SelectItem>
                <SelectItem value="image">Image</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      )}
    </div>
  )
}

function isQuickFilterActive(filter: TrafficFilter, quickFilter: QuickFilter): boolean {
  if (quickFilter.filter.statusCode && filter.statusCode === quickFilter.filter.statusCode) {
    return true
  }
  if (quickFilter.filter.minDuration && filter.minDuration === quickFilter.filter.minDuration) {
    return true
  }
  if (quickFilter.filter.url && filter.url === quickFilter.filter.url) {
    return true
  }
  return false
}
