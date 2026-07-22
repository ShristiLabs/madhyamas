import { useMemo, useState } from 'react'
import Prism from 'prismjs'
import 'prismjs/components/prism-json'
import 'prismjs/themes/prism-tomorrow.css'
import { JsonView as ReactJsonView, darkStyles } from 'react-json-view-lite'
import 'react-json-view-lite/dist/index.css'
import { Braces, Code2, Copy, Minimize2, Maximize2, Search } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

interface JsonViewProps {
  data: unknown
  searchTerm?: string
}

type ViewMode = 'code' | 'tree'

export function JsonView({ data, searchTerm }: JsonViewProps) {
  const [viewMode, setViewMode] = useState<ViewMode>('code')
  const [isPrettified, setIsPrettified] = useState(true)
  const [search, setSearch] = useState(searchTerm ?? '')

  const jsonData = data as Record<string, unknown> | unknown[]

  // Render JSON string (prettified or minified)
  const jsonStr = useMemo(() => {
    if (data === null || data === undefined) return ''
    return isPrettified
      ? JSON.stringify(data, null, 2)
      : JSON.stringify(data)
  }, [data, isPrettified])

  // Syntax-highlight the JSON string with Prism
  const highlightedHtml = useMemo(() => {
    if (!jsonStr) return ''
    try {
      return Prism.highlight(jsonStr, Prism.languages.json, 'json')
    } catch {
      // Fallback: escape HTML
      return jsonStr
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
    }
  }, [jsonStr])

  // Check if search term matches anything in the data
  const hasMatch = useMemo(() => {
    if (!search) return true
    const searchLower = search.toLowerCase()
    const checkValue = (value: unknown): boolean => {
      if (typeof value === 'string') {
        return value.toLowerCase().includes(searchLower)
      }
      if (typeof value === 'number' || typeof value === 'boolean') {
        return value.toString().toLowerCase().includes(searchLower)
      }
      if (Array.isArray(value)) {
        return value.some(checkValue)
      }
      if (value && typeof value === 'object') {
        return Object.values(value).some(checkValue) ||
               Object.keys(value).some(key => key.toLowerCase().includes(searchLower))
      }
      return false
    }
    return checkValue(data)
  }, [data, search])

  const handleCopy = async () => {
    await navigator.clipboard.writeText(jsonStr)
  }

  // Highlight search term in the syntax-highlighted HTML
  const finalHtml = useMemo(() => {
    if (!search || !highlightedHtml) return highlightedHtml
    // Escape the search term for use in regex
    const escaped = search.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    // Only highlight in text content, not in HTML tags
    // Split by HTML tags and only process text nodes
    return highlightedHtml.replace(/(<[^>]*>)|([^<]+)/g, (_, tag, text) => {
      if (tag) return tag
      return text.replace(new RegExp(`(${escaped})`, 'gi'), '<mark style="background-color: rgba(250, 204, 21, 0.4); border-radius: 2px;">$1</mark>')
    })
  }, [highlightedHtml, search])

  if (search && !hasMatch) {
    return (
      <div className="space-y-2">
        <JsonToolbar
          viewMode={viewMode}
          setViewMode={setViewMode}
          isPrettified={isPrettified}
          setIsPrettified={setIsPrettified}
          search={search}
          setSearch={setSearch}
          onCopy={handleCopy}
        />
        <div className="text-muted-foreground text-sm italic p-3">
          No matches found for &quot;{search}&quot;
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-2">
      <JsonToolbar
        viewMode={viewMode}
        setViewMode={setViewMode}
        isPrettified={isPrettified}
        setIsPrettified={setIsPrettified}
        search={search}
        setSearch={setSearch}
        onCopy={handleCopy}
      />

      {viewMode === 'code' ? (
        <pre
          className="font-mono text-sm bg-[#2d2d2d] text-[#ccc] p-3 rounded-md overflow-x-auto whitespace-pre"
          style={{ margin: 0 }}
        >
          <code
            className="language-json"
            dangerouslySetInnerHTML={{ __html: finalHtml || jsonStr }}
          />
        </pre>
      ) : (
        <div className="font-mono text-sm bg-muted rounded-md p-3 overflow-x-auto">
          <ReactJsonView
            data={jsonData}
            style={{
              ...darkStyles,
              container: 'bg-transparent',
              label: 'text-primary font-semibold',
              punctuation: 'text-foreground font-bold',
              stringValue: 'text-green-600 dark:text-green-400',
              numberValue: 'text-blue-600 dark:text-blue-400',
              booleanValue: 'text-purple-600 dark:text-purple-400',
              nullValue: 'text-red-600 dark:text-red-400',
            }}
          />
        </div>
      )}
    </div>
  )
}

interface JsonToolbarProps {
  viewMode: ViewMode
  setViewMode: (mode: ViewMode) => void
  isPrettified: boolean
  setIsPrettified: (v: boolean) => void
  search: string
  setSearch: (v: string) => void
  onCopy: () => void
}

function JsonToolbar({
  viewMode,
  setViewMode,
  isPrettified,
  setIsPrettified,
  search,
  setSearch,
  onCopy,
}: JsonToolbarProps) {
  return (
    <div className="flex items-center gap-2 flex-wrap">
      {/* View mode toggle */}
      <div className="flex items-center gap-1 bg-muted rounded-md p-0.5">
        <Button
          variant={viewMode === 'code' ? 'default' : 'ghost'}
          size="sm"
          className="h-7"
          onClick={() => setViewMode('code')}
          title="Code view with syntax highlighting"
        >
          <Code2 className="h-3.5 w-3.5 mr-1" />
          Code
        </Button>
        <Button
          variant={viewMode === 'tree' ? 'default' : 'ghost'}
          size="sm"
          className="h-7"
          onClick={() => setViewMode('tree')}
          title="Tree view (collapsible)"
        >
          <Braces className="h-3.5 w-3.5 mr-1" />
          Tree
        </Button>
      </div>

      {/* Prettify / Minify (only relevant in code view) */}
      {viewMode === 'code' && (
        <Button
          variant="outline"
          size="sm"
          className="h-7"
          onClick={() => setIsPrettified(!isPrettified)}
          title={isPrettified ? 'Minify JSON' : 'Prettify JSON'}
        >
          {isPrettified ? (
            <>
              <Minimize2 className="h-3.5 w-3.5 mr-1" />
              Minify
            </>
          ) : (
            <>
              <Maximize2 className="h-3.5 w-3.5 mr-1" />
              Prettify
            </>
          )}
        </Button>
      )}

      {/* Search */}
      <div className="relative flex-1 min-w-[120px] max-w-xs">
        <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
        <Input
          placeholder="Search JSON..."
          className="pl-7 h-7 text-sm"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      {/* Copy */}
      <Button variant="ghost" size="sm" className="h-7" onClick={onCopy}>
        <Copy className="h-3.5 w-3.5 mr-1" />
        Copy
      </Button>
    </div>
  )
}
