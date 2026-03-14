import { useState, useMemo, useCallback } from 'react'
import { Tabs, TabsContent, TabsList, TabsTrigger } from './ui/tabs'
import { ScrollArea } from './ui/scroll-area'
import { Button } from './ui/button'
import {
  Copy,
  Download,
  Code2,
  FileText,
  Braces,
  Search,
  Minimize2,
  Maximize2,
  ChevronDown,
} from 'lucide-react'
import { JsonView } from './JsonView'
import { HeadersView } from './HeadersView'
import { Input } from './ui/input'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './ui/dropdown-menu'
import type { TrafficEntry } from '@/types/traffic'
import { useToast } from './ui/use-toast'

interface TrafficDetailProps {
  entry: TrafficEntry
}

export function TrafficDetail({ entry }: TrafficDetailProps) {
  const { toast } = useToast()
  const [activeTab, setActiveTab] = useState('headers')

  const handleCopyCurl = useCallback(async () => {
    const curl = generateCurl(entry)
    await navigator.clipboard.writeText(curl)
    toast({ description: 'cURL command copied to clipboard' })
  }, [entry, toast])

  const handleCopyHttpie = useCallback(async () => {
    const httpie = generateHttpie(entry)
    await navigator.clipboard.writeText(httpie)
    toast({ description: 'HTTPie command copied to clipboard' })
  }, [entry, toast])

  const handleCopyFetch = useCallback(async () => {
    const fetch = generateFetch(entry)
    await navigator.clipboard.writeText(fetch)
    toast({ description: 'Fetch code copied to clipboard' })
  }, [entry, toast])

  const handleCopyWget = useCallback(async () => {
    const wget = generateWget(entry)
    await navigator.clipboard.writeText(wget)
    toast({ description: 'wget command copied to clipboard' })
  }, [entry, toast])

  const handleExport = useCallback(() => {
    const data = JSON.stringify(entry, null, 2)
    const blob = new Blob([data], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `request-${entry.id}.json`
    a.click()
    URL.revokeObjectURL(url)
  }, [entry])

  const handleExportHAR = useCallback(() => {
    const har = generateHAR(entry)
    const blob = new Blob([har], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `request-${entry.id}.har`
    a.click()
    URL.revokeObjectURL(url)
    toast({ description: 'HAR file exported' })
  }, [entry, toast])

  return (
    <div className="flex flex-col h-full">
      <div className="px-4 py-3 border-b bg-muted/50">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <span className={`font-mono font-bold method-${entry.request.method.toLowerCase()}`}>
              {entry.request.method}
            </span>
            <span className="text-sm text-muted-foreground">
              {entry.response?.status_code || 'Pending'}
            </span>
            {entry.response && (
              <span className="text-xs text-muted-foreground">
                {entry.response.duration_ms}ms
              </span>
            )}
          </div>
          <div className="flex gap-2">
            {/* Export Options */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm">
                  <Copy className="h-4 w-4 mr-1" />
                  Copy as
                  <ChevronDown className="h-3 w-3 ml-1" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={handleCopyCurl}>
                  <Code2 className="h-4 w-4 mr-2" />
                  cURL
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleCopyHttpie}>
                  <Code2 className="h-4 w-4 mr-2" />
                  HTTPie
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleCopyFetch}>
                  <Braces className="h-4 w-4 mr-2" />
                  JavaScript Fetch
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleCopyWget}>
                  <Download className="h-4 w-4 mr-2" />
                  wget
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Export */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm">
                  <Download className="h-4 w-4 mr-1" />
                  Export
                  <ChevronDown className="h-3 w-3 ml-1" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={handleExport}>
                  <FileText className="h-4 w-4 mr-2" />
                  JSON
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleExportHAR}>
                  <FileText className="h-4 w-4 mr-2" />
                  HAR
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
        <div className="font-mono text-sm break-all">
          {entry.request.url}
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
        <div className="border-b px-4">
          <TabsList className="h-10">
            <TabsTrigger value="headers">Headers</TabsTrigger>
            <TabsTrigger value="request">Request</TabsTrigger>
            <TabsTrigger value="response">Response</TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value="headers" className="flex-1 overflow-hidden m-0" role="tabpanel">
          <ScrollArea className="h-full">
            <div className="p-4">
              <h4 className="font-semibold mb-2">Request Headers</h4>
              <HeadersView headers={entry.request.headers} />

              {entry.response && (
                <>
                  <h4 className="font-semibold mb-2 mt-4">Response Headers</h4>
                  <HeadersView headers={entry.response.headers} />
                </>
              )}
            </div>
          </ScrollArea>
        </TabsContent>

        <TabsContent value="request" className="flex-1 overflow-hidden m-0" role="tabpanel">
          <ScrollArea className="h-full">
            <div className="p-4">
              <BodyView
                body={entry.request.body}
                contentType={entry.request.content_type}
              />
            </div>
          </ScrollArea>
        </TabsContent>

        <TabsContent value="response" className="flex-1 overflow-hidden m-0" role="tabpanel">
          <ScrollArea className="h-full">
            <div className="p-4">
              {entry.response ? (
                <BodyView
                  body={entry.response.body}
                  contentType={entry.response.content_type}
                />
              ) : (
                <div className="text-muted-foreground">No response yet</div>
              )}
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </div>
  )
}

interface BodyViewProps {
  body?: string
  contentType?: string
}

function BodyView({ body, contentType }: BodyViewProps) {
  const [isPrettified, setIsPrettified] = useState(true)
  const [searchTerm, setSearchTerm] = useState('')

  // Calculate values before early return
  const isJson = body
    ? (contentType?.includes('application/json') || body.startsWith('{') || body.startsWith('['))
    : false

  const parsedJson = useMemo(() => {
    if (!body || !isJson) return null
    try {
      return JSON.parse(body)
    } catch {
      return null
    }
  }, [isJson, body])

  const displayBody = useMemo(() => {
    if (!body) return ''
    if (isJson && parsedJson) {
      return isPrettified ? JSON.stringify(parsedJson, null, 2) : JSON.stringify(parsedJson)
    }
    return body
  }, [isJson, parsedJson, isPrettified, body])

  const handleCopy = async () => {
    await navigator.clipboard.writeText(displayBody)
  }

  // Early return after all hooks
  if (!body) {
    return <div className="text-muted-foreground">No body</div>
  }

  return (
    <div className="space-y-2">
      {/* Toolbar */}
      <div className="flex items-center gap-2 flex-wrap">
        {isJson && parsedJson && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setIsPrettified(!isPrettified)}
            title={isPrettified ? 'Minify' : 'Prettify'}
          >
            {isPrettified ? (
              <>
                <Minimize2 className="h-4 w-4 mr-1" />
                Minify
              </>
            ) : (
              <>
                <Maximize2 className="h-4 w-4 mr-1" />
                Prettify
              </>
            )}
          </Button>
        )}
        <div className="relative flex-1 max-w-xs">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search body..."
            className="pl-8 h-8"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>
        <Button variant="ghost" size="sm" onClick={handleCopy}>
          <Copy className="h-4 w-4 mr-1" />
          Copy
        </Button>
      </div>

      {/* Body Content */}
      {isJson && parsedJson ? (
        <JsonView data={parsedJson} searchTerm={searchTerm} />
      ) : (
        <pre className="font-mono text-sm bg-muted p-3 rounded-md overflow-x-auto whitespace-pre-wrap break-all">
          {highlightBodyText(displayBody, searchTerm)}
        </pre>
      )}
    </div>
  )
}

function highlightBodyText(text: string, searchTerm: string): React.ReactNode {
  if (!searchTerm) return text

  const parts = text.split(new RegExp(`(${escapeRegex(searchTerm)})`, 'gi'))
  return parts.map((part, i) =>
    part.toLowerCase() === searchTerm.toLowerCase() ? (
      <mark key={i} className="bg-yellow-200 dark:bg-yellow-800 rounded px-0.5">
        {part}
      </mark>
    ) : (
      part
    )
  )
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function generateCurl(entry: TrafficEntry): string {
  const req = entry.request
  let cmd = `curl -X ${req.method} '${req.url}'`

  for (const [key, value] of Object.entries(req.headers)) {
    if (!['host', 'content-length', 'connection'].includes(key.toLowerCase())) {
      cmd += ` \\\n  -H '${key}: ${value.replace(/'/g, "'\\''")}'`
    }
  }

  if (req.body) {
    cmd += ` \\\n  -d '${req.body.replace(/'/g, "'\\''")}'`
  }

  return cmd
}

function generateHttpie(entry: TrafficEntry): string {
  const req = entry.request
  let cmd = `http ${req.method} '${req.url}'`

  for (const [key, value] of Object.entries(req.headers)) {
    if (!['host', 'content-length', 'connection'].includes(key.toLowerCase())) {
      cmd += ` \\\n  '${key}: ${value.replace(/'/g, "'\\''")}'`
    }
  }

  if (req.body) {
    try {
      const json = JSON.parse(req.body)
      for (const [key, value] of Object.entries(json)) {
        cmd += ` \\\n  ${key}=${JSON.stringify(value)}`
      }
    } catch {
      cmd += ` \\\n  <<< '${req.body.replace(/'/g, "'\\''")}'`
    }
  }

  return cmd
}

function generateFetch(entry: TrafficEntry): string {
  const req = entry.request
  const headers: Record<string, string> = {}

  for (const [key, value] of Object.entries(req.headers)) {
    if (!['host', 'content-length', 'connection'].includes(key.toLowerCase())) {
      headers[key] = value
    }
  }

  const options: Record<string, unknown> = {
    method: req.method,
    headers,
  }

  if (req.body) {
    options.body = req.body
  }

  return `fetch('${req.url}', ${JSON.stringify(options, null, 2)})
  .then(response => response.json())
  .then(data => console.log(data))
  .catch(error => console.error('Error:', error));`
}

function generateWget(entry: TrafficEntry): string {
  const req = entry.request
  let cmd = `wget --method=${req.method} '${req.url}'`

  for (const [key, value] of Object.entries(req.headers)) {
    if (!['host', 'content-length', 'connection'].includes(key.toLowerCase())) {
      cmd += ` \\\n  --header='${key}: ${value.replace(/'/g, "'\\''")}'`
    }
  }

  if (req.body) {
    cmd += ` \\\n  --body-data='${req.body.replace(/'/g, "'\\''")}'`
  }

  cmd += ' -O -'

  return cmd
}

function generateHAR(entry: TrafficEntry): string {
  const har = {
    log: {
      version: '1.2',
      creator: { name: 'ProxyForge', version: '0.1.0' },
      entries: [
        {
          startedDateTime: new Date(entry.timestamp).toISOString(),
          request: {
            method: entry.request.method,
            url: entry.request.url,
            httpVersion: 'HTTP/1.1',
            headers: Object.entries(entry.request.headers).map(([name, value]) => ({
              name,
              value,
            })),
            queryString: [],
            postData: entry.request.body
              ? { mimeType: entry.request.content_type || 'text/plain', text: entry.request.body }
              : undefined,
            headersSize: -1,
            bodySize: entry.request.body?.length || 0,
          },
          response: entry.response
            ? {
                status: entry.response.status_code,
                statusText: entry.response.status_message || '',
                httpVersion: 'HTTP/1.1',
                headers: Object.entries(entry.response.headers).map(([name, value]) => ({
                  name,
                  value,
                })),
                content: {
                  size: entry.response.body?.length || 0,
                  mimeType: entry.response.content_type || 'text/plain',
                  text: entry.response.body || '',
                },
                headersSize: -1,
                bodySize: entry.response.body?.length || 0,
              }
            : undefined,
          time: entry.response?.duration_ms || 0,
        },
      ],
    },
  }

  return JSON.stringify(har, null, 2)
}
