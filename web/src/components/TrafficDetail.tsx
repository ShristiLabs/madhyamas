import { useState, useMemo, useCallback } from "react";
import { Checkbox } from "./ui/checkbox";
import { Label } from "./ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Button } from "./ui/button";
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
} from "lucide-react";
import { JsonView } from "./JsonView";
import { Input } from "./ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import type { TrafficEntry } from "@/types/traffic";
import { useToast } from "./ui/use-toast";

interface TrafficDetailProps {
  entry: TrafficEntry;
}

export function TrafficDetail({ entry }: TrafficDetailProps) {
  const { toast } = useToast();
  const [activeTab, setActiveTab] = useState("request");
  const [decodeRequest, setDecodeRequest] = useState(false);
  const [decodeResponse, setDecodeResponse] = useState(false);

  const handleCopyCurl = useCallback(async () => {
    const curl = generateCurl(entry);
    await navigator.clipboard.writeText(curl);
    toast({ description: "cURL command copied to clipboard" });
  }, [entry, toast]);

  const handleCopyHttpie = useCallback(async () => {
    const httpie = generateHttpie(entry);
    await navigator.clipboard.writeText(httpie);
    toast({ description: "HTTPie command copied to clipboard" });
  }, [entry, toast]);

  const handleCopyFetch = useCallback(async () => {
    const fetch = generateFetch(entry);
    await navigator.clipboard.writeText(fetch);
    toast({ description: "Fetch code copied to clipboard" });
  }, [entry, toast]);

  const handleCopyWget = useCallback(async () => {
    const wget = generateWget(entry);
    await navigator.clipboard.writeText(wget);
    toast({ description: "wget command copied to clipboard" });
  }, [entry, toast]);

  const handleExport = useCallback(() => {
    const data = JSON.stringify(entry, null, 2);
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `request-${entry.id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }, [entry]);

  const handleExportHAR = useCallback(() => {
    const har = generateHAR(entry);
    const blob = new Blob([har], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `request-${entry.id}.har`;
    a.click();
    URL.revokeObjectURL(url);
    toast({ description: "HAR file exported" });
  }, [entry, toast]);

  return (
    <div className="flex flex-col flex-1 min-h-0 overflow-y-auto">
      <div className="px-4 py-3 border-b bg-muted/50">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <span
              className={`font-mono font-bold method-${entry.request.method.toLowerCase()}`}
            >
              {entry.request.method}
            </span>
            <span className="text-sm text-muted-foreground">
              {entry.response?.status_code || "Pending"}
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
        <div className="font-mono text-sm truncate">{entry.request.url}</div>
      </div>

      <Tabs
        value={activeTab}
        onValueChange={setActiveTab}
        className="flex-1 flex flex-col"
      >
        <div className="border-b px-4">
          <TabsList className="h-10">
            <TabsTrigger value="request">Request</TabsTrigger>
            <TabsTrigger value="response">Response</TabsTrigger>
            <TabsTrigger value="timing">Timing</TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value="request" className="m-0" role="tabpanel">
          <div className="p-4 space-y-4">
            {/* URL Section */}
            <div>
              <h4 className="font-semibold mb-2 text-sm">URL</h4>
              <div className="font-mono text-sm bg-muted p-2 rounded-md break-all">
                {entry.request.url}
              </div>
            </div>

            {/* Request Headers Table */}
            <div>
              <h4 className="font-semibold mb-2 text-sm">Headers</h4>
              <HeadersTable headers={entry.request.headers} />
            </div>

            {/* Request Body */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <h4 className="font-semibold text-sm">Body</h4>
                {entry.request.body && entry.request.body.length > 0 && (
                  <div className="flex items-center space-x-2">
                    <Checkbox
                      id="decode-request"
                      checked={decodeRequest}
                      onCheckedChange={(checked) =>
                        setDecodeRequest(checked as boolean)
                      }
                    />
                    <Label
                      htmlFor="decode-request"
                      className="text-sm cursor-pointer"
                    >
                      Decode payload
                    </Label>
                  </div>
                )}
              </div>
              <BodyView
                body={entry.request.body}
                contentType={entry.request.content_type}
                decode={decodeRequest}
              />
            </div>
          </div>
        </TabsContent>

        <TabsContent value="response" className="m-0" role="tabpanel">
          <div className="p-4 space-y-4">
            {entry.response ? (
              <>
                {/* HTTP Status */}
                <div>
                  <h4 className="font-semibold mb-2 text-sm">Status</h4>
                  <div className="flex items-center gap-2">
                    <span
                      className={`font-mono font-bold px-2 py-1 rounded text-sm ${
                        entry.response.status_code >= 200 &&
                        entry.response.status_code < 300
                          ? "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
                          : entry.response.status_code >= 300 &&
                              entry.response.status_code < 400
                            ? "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200"
                            : entry.response.status_code >= 400 &&
                                entry.response.status_code < 500
                              ? "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200"
                              : "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
                      }`}
                    >
                      {entry.response.status_code}{" "}
                      {entry.response.status_message || ""}
                    </span>
                  </div>
                </div>

                {/* Response Headers Table */}
                <div>
                  <h4 className="font-semibold mb-2 text-sm">Headers</h4>
                  <HeadersTable headers={entry.response.headers} />
                </div>

                {/* Response Body */}
                <div>
                  <div className="flex items-center justify-between mb-2">
                    <h4 className="font-semibold text-sm">Body</h4>
                    {entry.response.body && entry.response.body.length > 0 && (
                      <div className="flex items-center space-x-2">
                        <Checkbox
                          id="decode-response"
                          checked={decodeResponse}
                          onCheckedChange={(checked) =>
                            setDecodeResponse(checked as boolean)
                          }
                        />
                        <Label
                          htmlFor="decode-response"
                          className="text-sm cursor-pointer"
                        >
                          Decode payload
                        </Label>
                      </div>
                    )}
                  </div>
                  <BodyView
                    body={entry.response.body}
                    contentType={entry.response.content_type}
                    decode={decodeResponse}
                  />
                </div>
              </>
            ) : (
              <div className="text-muted-foreground">No response yet</div>
            )}
          </div>
        </TabsContent>

        <TabsContent value="timing" className="m-0" role="tabpanel">
          <div className="p-4 space-y-4">
            <div>
              <h4 className="font-semibold mb-3 text-sm">Request Timing</h4>
              <div className="space-y-2">
                <div className="flex justify-between items-center py-2 border-b">
                  <span className="text-sm text-muted-foreground">
                    Timestamp
                  </span>
                  <span className="font-mono text-sm">
                    {new Date(entry.timestamp).toLocaleString()}
                  </span>
                </div>
                {entry.response && (
                  <>
                    <div className="flex justify-between items-center py-2 border-b">
                      <span className="text-sm text-muted-foreground">
                        Duration
                      </span>
                      <span className="font-mono text-sm font-semibold">
                        {entry.response.duration_ms}ms
                      </span>
                    </div>
                    <div className="flex justify-between items-center py-2 border-b">
                      <span className="text-sm text-muted-foreground">
                        Request Size
                      </span>
                      <span className="font-mono text-sm">
                        {formatBytes(entry.request.body?.length || 0)}
                      </span>
                    </div>
                    <div className="flex justify-between items-center py-2 border-b">
                      <span className="text-sm text-muted-foreground">
                        Response Size
                      </span>
                      <span className="font-mono text-sm">
                        {formatBytes(entry.response.body?.length || 0)}
                      </span>
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

interface BodyViewProps {
  body?: string;
  contentType?: string;
  decode?: boolean;
}

function HeadersTable({ headers }: { headers: Record<string, string> }) {
  const entries = Object.entries(headers);

  if (entries.length === 0) {
    return <div className="text-muted-foreground text-sm">No headers</div>;
  }

  return (
    <div className="border rounded-md overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="bg-muted/50 border-b">
            <th className="text-left p-2 font-semibold">Name</th>
            <th className="text-left p-2 font-semibold">Value</th>
          </tr>
        </thead>
        <tbody>
          {entries.map(([key, value], index) => (
            <tr
              key={key}
              className={index % 2 === 0 ? "bg-background" : "bg-muted/20"}
            >
              <td className="p-2 font-mono text-primary font-medium align-top">
                {key}
              </td>
              <td className="p-2 font-mono text-muted-foreground break-all">
                {value}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + " " + sizes[i];
}

function BodyView({ body, contentType, decode = false }: BodyViewProps) {
  const [isPrettified, setIsPrettified] = useState(true);
  const [searchTerm, setSearchTerm] = useState("");

  // Decode base64-prefixed bodies (backend marks binary/non-UTF-8 bodies
  // with "base64:" prefix). Also handles the "Decode payload" checkbox.
  const decodedBody = useMemo(() => {
    if (!body) return "";

    // If the body has the "base64:" prefix, decode it automatically.
    // The backend uses this prefix for content that isn't valid UTF-8
    // (e.g. binary data, or compressed content that wasn't decompressed).
    if (body.startsWith("base64:")) {
      const b64Data = body.slice(7);
      try {
        // atob() returns a binary string; convert to UTF-8 safely
        const binaryStr = atob(b64Data);
        // Convert binary string to UTF-8 string
        const bytes = new Uint8Array(binaryStr.length);
        for (let i = 0; i < binaryStr.length; i++) {
          bytes[i] = binaryStr.charCodeAt(i);
        }
        try {
          return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
        } catch {
          // If UTF-8 decode fails, show the raw binary string
          return binaryStr;
        }
      } catch {
        return body; // return original if base64 decode fails
      }
    }

    // "Decode payload" checkbox: try base64 decode on the raw body
    if (decode) {
      try {
        return atob(body);
      } catch {
        // not valid base64, return as-is
      }
    }

    return body;
  }, [body, decode]);

  // Calculate values before early return
  const isJson = decodedBody
    ? contentType?.includes("application/json") ||
      decodedBody.startsWith("{") ||
      decodedBody.startsWith("[")
    : false;

  const parsedJson = useMemo(() => {
    if (!decodedBody || !isJson) return null;
    try {
      return JSON.parse(decodedBody);
    } catch {
      return null;
    }
  }, [isJson, decodedBody]);

  const displayBody = useMemo(() => {
    if (!decodedBody) return "";

    if (isJson && parsedJson) {
      return isPrettified
        ? JSON.stringify(parsedJson, null, 2)
        : JSON.stringify(parsedJson);
    }
    return decodedBody;
  }, [isJson, parsedJson, isPrettified, decodedBody]);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(displayBody);
  };

  // Early return after all hooks
  if (!body) {
    return <div className="text-muted-foreground">No body</div>;
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
            title={isPrettified ? "Minify" : "Prettify"}
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
  );
}

function highlightBodyText(text: string, searchTerm: string): React.ReactNode {
  if (!searchTerm) return text;

  const parts = text.split(new RegExp(`(${escapeRegex(searchTerm)})`, "gi"));
  return parts.map((part, i) =>
    part.toLowerCase() === searchTerm.toLowerCase() ? (
      <mark key={i} className="bg-yellow-200 dark:bg-yellow-800 rounded px-0.5">
        {part}
      </mark>
    ) : (
      part
    ),
  );
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function generateCurl(entry: TrafficEntry): string {
  const req = entry.request;
  let cmd = `curl -X ${req.method} '${req.url}'`;

  for (const [key, value] of Object.entries(req.headers)) {
    if (!["host", "content-length", "connection"].includes(key.toLowerCase())) {
      cmd += ` \\\n  -H '${key}: ${value.replace(/'/g, "'\\''")}'`;
    }
  }

  if (req.body) {
    cmd += ` \\\n  -d '${req.body.replace(/'/g, "'\\''")}'`;
  }

  return cmd;
}

function generateHttpie(entry: TrafficEntry): string {
  const req = entry.request;
  let cmd = `http ${req.method} '${req.url}'`;

  for (const [key, value] of Object.entries(req.headers)) {
    if (!["host", "content-length", "connection"].includes(key.toLowerCase())) {
      cmd += ` \\\n  '${key}: ${value.replace(/'/g, "'\\''")}'`;
    }
  }

  if (req.body) {
    try {
      const json = JSON.parse(req.body);
      for (const [key, value] of Object.entries(json)) {
        cmd += ` \\\n  ${key}=${JSON.stringify(value)}`;
      }
    } catch {
      cmd += ` \\\n  <<< '${req.body.replace(/'/g, "'\\''")}'`;
    }
  }

  return cmd;
}

function generateFetch(entry: TrafficEntry): string {
  const req = entry.request;
  const headers: Record<string, string> = {};

  for (const [key, value] of Object.entries(req.headers)) {
    if (!["host", "content-length", "connection"].includes(key.toLowerCase())) {
      headers[key] = value;
    }
  }

  const options: Record<string, unknown> = {
    method: req.method,
    headers,
  };

  if (req.body) {
    options.body = req.body;
  }

  return `fetch('${req.url}', ${JSON.stringify(options, null, 2)})
  .then(response => response.json())
  .then(data => console.log(data))
  .catch(error => console.error('Error:', error));`;
}

function generateWget(entry: TrafficEntry): string {
  const req = entry.request;
  let cmd = `wget --method=${req.method} '${req.url}'`;

  for (const [key, value] of Object.entries(req.headers)) {
    if (!["host", "content-length", "connection"].includes(key.toLowerCase())) {
      cmd += ` \\\n  --header='${key}: ${value.replace(/'/g, "'\\''")}'`;
    }
  }

  if (req.body) {
    cmd += ` \\\n  --body-data='${req.body.replace(/'/g, "'\\''")}'`;
  }

  cmd += " -O -";

  return cmd;
}

function generateHAR(entry: TrafficEntry): string {
  const har = {
    log: {
      version: "1.2",
      creator: { name: "Madhyamas", version: "0.1.0" },
      entries: [
        {
          startedDateTime: new Date(entry.timestamp).toISOString(),
          request: {
            method: entry.request.method,
            url: entry.request.url,
            httpVersion: "HTTP/1.1",
            headers: Object.entries(entry.request.headers).map(
              ([name, value]) => ({
                name,
                value,
              }),
            ),
            queryString: [],
            postData: entry.request.body
              ? {
                  mimeType: entry.request.content_type || "text/plain",
                  text: entry.request.body,
                }
              : undefined,
            headersSize: -1,
            bodySize: entry.request.body?.length || 0,
          },
          response: entry.response
            ? {
                status: entry.response.status_code,
                statusText: entry.response.status_message || "",
                httpVersion: "HTTP/1.1",
                headers: Object.entries(entry.response.headers).map(
                  ([name, value]) => ({
                    name,
                    value,
                  }),
                ),
                content: {
                  size: entry.response.body?.length || 0,
                  mimeType: entry.response.content_type || "text/plain",
                  text: entry.response.body || "",
                },
                headersSize: -1,
                bodySize: entry.response.body?.length || 0,
              }
            : undefined,
          time: entry.response?.duration_ms || 0,
        },
      ],
    },
  };

  return JSON.stringify(har, null, 2);
}
