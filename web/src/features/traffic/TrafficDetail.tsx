import { useState, useMemo, useCallback, useEffect } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import {
  Copy,
  Download,
  Code2,
  FileText,
  Braces,
  Search,
  ChevronDown,
  FileArchive,
  Eye,
  EyeOff,
  Terminal,
  X,
} from "lucide-react";
import { JSONPath } from "jsonpath-plus";
import jmespath from "jmespath";
import { JsonView } from "@/features/traffic/JsonView";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { TrafficEntry } from "@/types/traffic";
import { apiGet } from "@/lib/api/client";
import { useToast } from "@/components/ui/use-toast";

interface TrafficDetailProps {
  entry: TrafficEntry;
}

export function TrafficDetail({ entry }: TrafficDetailProps) {
  const { toast } = useToast();
  const [activeTab, setActiveTab] = useState("request");

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

            {/* HTTP Version */}
            {entry.request.http_version && (
              <div>
                <h4 className="font-semibold mb-2 text-sm">HTTP Version</h4>
                <div className="font-mono text-sm bg-muted p-2 rounded-md">
                  {entry.request.http_version}
                </div>
              </div>
            )}

            {/* Request Headers Table */}
            <div>
              <h4 className="font-semibold mb-2 text-sm">Headers</h4>
              <HeadersTable headers={entry.request.headers} />
            </div>

            {/* Request Body */}
            <div>
              <h4 className="font-semibold mb-2 text-sm">Body</h4>
              <BodyView
                body={entry.request.body}
                contentType={entry.request.content_type}
                headers={entry.request.headers}
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

                {/* HTTP Version (response) */}
                {entry.response.http_version && (
                  <div>
                    <h4 className="font-semibold mb-2 text-sm">HTTP Version</h4>
                    <div className="font-mono text-sm bg-muted p-2 rounded-md">
                      {entry.response.http_version}
                    </div>
                  </div>
                )}

                {/* Response Headers Table */}
                <div>
                  <h4 className="font-semibold mb-2 text-sm">Headers</h4>
                  <HeadersTable headers={entry.response.headers} />
                </div>

                {/* Response Body */}
                <div>
                  <h4 className="font-semibold mb-2 text-sm">Body</h4>
                  <BodyView
                    body={entry.response.body}
                    contentType={entry.response.content_type}
                    headers={entry.response.headers}
                    entryId={entry.id}
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
              <MiniWaterfall entry={entry} />
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
                        {formatBytes(
                          entry.request_size ?? entry.request.body?.length ?? 0
                        )}
                      </span>
                    </div>
                    <div className="flex justify-between items-center py-2 border-b">
                      <span className="text-sm text-muted-foreground">
                        Response Size
                      </span>
                      <span className="font-mono text-sm">
                        {formatBytes(
                          entry.response_size ??
                            entry.response?.body?.length ??
                            0
                        )}
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
  headers?: Record<string, string>;
  entryId?: string;
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

function miniBarColor(statusCode?: number): string {
  if (!statusCode) return "bg-muted-foreground/40";
  const cls = Math.floor(statusCode / 100);
  switch (cls) {
    case 2:
      return "bg-success/70";
    case 3:
      return "bg-primary/70";
    case 4:
      return "bg-warning/70";
    case 5:
      return "bg-destructive/70";
    default:
      return "bg-muted-foreground/40";
  }
}

function MiniWaterfall({ entry }: { entry: TrafficEntry }) {
  const duration = entry.response?.duration_ms ?? 0;
  const hasResponse = !!entry.response;
  const barColor = miniBarColor(entry.response?.status_code);
  const scaleMax = Math.max(duration, 100);
  const barPct = hasResponse ? Math.max((duration / scaleMax) * 100, 1) : 0;

  return (
    <div className="mb-4 rounded-md border border-border bg-muted/20 p-3">
      <div className="mb-1.5 flex items-center justify-between text-2xs text-muted-foreground">
        <span>Duration</span>
        <span className="font-mono">
          {hasResponse ? `${duration}ms` : "Pending"}
        </span>
      </div>
      <div className="relative h-4 w-full overflow-hidden rounded-sm bg-muted/40">
        <div
          className={`absolute top-0 left-0 h-full rounded-sm ${barColor}`}
          style={{ width: `${barPct}%` }}
        />
      </div>
      <div className="mt-1 flex justify-between text-2xs text-muted-foreground">
        <span>0ms</span>
        <span>{scaleMax >= 1000 ? `${(scaleMax / 1000).toFixed(1)}s` : `${scaleMax}ms`}</span>
      </div>
    </div>
  );
}

type JsonQueryMode = "none" | "jsonpath" | "jmespath";

function BodyView({ body, contentType, headers, entryId }: BodyViewProps) {
  const [searchTerm, setSearchTerm] = useState("");
  const [showDecompressed, setShowDecompressed] = useState(true);
  const [showAsImage, setShowAsImage] = useState(true);
  const [queryMode, setQueryMode] = useState<JsonQueryMode>("none");
  const [queryString, setQueryString] = useState("");

  // Check for Content-Encoding header (case-insensitive)
  const contentEncoding = useMemo(() => {
    if (!headers) return undefined;
    const entry = Object.entries(headers).find(([k]) =>
      k.toLowerCase() === "content-encoding",
    );
    return entry?.[1];
  }, [headers]);

  const isCompressed = !!contentEncoding;

  // zstd is not supported by the browser's DecompressionStream API, so it
  // must be decompressed by the backend via the ?decompressed=true endpoint.
  const isZstd = useMemo(
    () => contentEncoding?.toLowerCase().trim() === "zstd",
    [contentEncoding],
  );

  // Detect image content types
  const isImage = useMemo(() => {
    if (!contentType) return false;
    const ct = contentType.toLowerCase();
    return ct.startsWith("image/");
  }, [contentType]);

  // Decode body: handles base64 prefix, and async decompression when
  // Content-Encoding is present and the user wants the decompressed view.
  // Tracks both the text representation (for text display) and the raw
  // bytes (for image data URLs and downloads).
  const [decodedBody, setDecodedBody] = useState("");
  const [rawBytes, setRawBytes] = useState<Uint8Array | null>(null);

  useEffect(() => {
    if (!body) {
      setDecodedBody("");
      setRawBytes(null);
      return;
    }

    const bodyStr = body;
    let cancelled = false;

    async function decode() {
      // Step 1: Get raw bytes from the body string (may be base64-encoded)
      let bytes: Uint8Array;
      if (bodyStr.startsWith("base64:")) {
        const b64Data = bodyStr.slice(7);
        try {
          const binaryStr = atob(b64Data);
          bytes = new Uint8Array(binaryStr.length);
          for (let i = 0; i < binaryStr.length; i++) {
            bytes[i] = binaryStr.charCodeAt(i);
          }
        } catch {
          if (!cancelled) {
            setDecodedBody(bodyStr);
            setRawBytes(null);
          }
          return;
        }
      } else {
        bytes = new TextEncoder().encode(bodyStr);
      }

      // Step 2: If compressed and decompression is requested, use
      // DecompressionStream to decompress the raw bytes. For zstd (not
      // supported by the browser), fetch the decompressed body from the
      // backend via the ?decompressed=true endpoint.
      if (isCompressed && showDecompressed && contentEncoding) {
        if (isZstd && entryId) {
          // zstd is not supported by DecompressionStream — fetch the
          // decompressed body from the backend.
          try {
            const decompressedEntry = await apiGet<TrafficEntry>(
              `/traffic/${entryId}?decompressed=true`,
            );
            if (cancelled) return;
            const decompressedBody = decompressedEntry.response?.body;
            if (decompressedBody) {
              let decBytes: Uint8Array;
              if (decompressedBody.startsWith("base64:")) {
                const b64Data = decompressedBody.slice(7);
                const binaryStr = atob(b64Data);
                decBytes = new Uint8Array(binaryStr.length);
                for (let i = 0; i < binaryStr.length; i++) {
                  decBytes[i] = binaryStr.charCodeAt(i);
                }
              } else {
                decBytes = new TextEncoder().encode(decompressedBody);
              }
              setRawBytes(decBytes);
              setDecodedBody(
                new TextDecoder("utf-8", { fatal: false }).decode(decBytes),
              );
            } else {
              if (!cancelled) {
                setRawBytes(bytes);
                setDecodedBody(
                  `[Decompression returned empty body, encoding: ${contentEncoding}]`,
                );
              }
            }
          } catch {
            if (!cancelled) {
              setRawBytes(bytes);
              setDecodedBody(
                `[Decompression failed — showing raw data, ${bytes.length} bytes, encoding: ${contentEncoding}]`,
              );
            }
          }
        } else {
          const encoding = contentEncoding.toLowerCase().trim();
          try {
            const format = encoding === "x-gzip" ? "gzip" : encoding;
            const ds = new DecompressionStream(format as CompressionFormat);
            const blob = new Blob([bytes.buffer as ArrayBuffer]);
            const stream = blob.stream().pipeThrough(ds);
            const buffer = await new Response(stream).arrayBuffer();
            if (cancelled) return;
            const decompressed = new Uint8Array(buffer);
            setRawBytes(decompressed);
            setDecodedBody(
              new TextDecoder("utf-8", { fatal: false }).decode(decompressed),
            );
          } catch {
            // Decompression failed — fall back to showing raw bytes
            if (!cancelled) {
              setRawBytes(bytes);
              setDecodedBody(
                `[Decompression failed — showing raw data, ${bytes.length} bytes, encoding: ${contentEncoding}]`,
              );
            }
          }
        }
      } else if (isCompressed && !showDecompressed) {
        // Show raw compressed data info
        if (!cancelled) {
          setRawBytes(bytes);
          setDecodedBody(
            `[Raw compressed data: ${bytes.length} bytes, encoding: ${contentEncoding}]`,
          );
        }
      } else {
        // Not compressed — decode bytes as UTF-8
        if (!cancelled) {
          setRawBytes(bytes);
          try {
            setDecodedBody(
              new TextDecoder("utf-8", { fatal: false }).decode(bytes),
            );
          } catch {
            setDecodedBody(bodyStr);
          }
        }
      }
    }

    decode();
    return () => {
      cancelled = true;
    };
  }, [body, isCompressed, showDecompressed, contentEncoding, isZstd, entryId]);

  // Build a data URL for image display from raw bytes + content type.
  const imageDataUrl = useMemo(() => {
    if (!isImage || !rawBytes || rawBytes.length === 0) return undefined;
    // For SVG, the body may be text (not base64-prefixed), so use the
    // raw text directly with a data URL.
    if (contentType?.includes("svg")) {
      const svgText = new TextDecoder("utf-8", { fatal: false }).decode(
        rawBytes,
      );
      return `data:image/svg+xml;utf8,${encodeURIComponent(svgText)}`;
    }
    // For binary image formats, use base64 data URL. Build in chunks to
    // avoid call stack limits with String.fromCharCode(...spread) on large
    // images.
    const chunkSize = 8192;
    let binaryStr = "";
    for (let i = 0; i < rawBytes.length; i += chunkSize) {
      const chunk = rawBytes.subarray(i, Math.min(i + chunkSize, rawBytes.length));
      binaryStr += String.fromCharCode.apply(null, Array.from(chunk));
    }
    const b64 = btoa(binaryStr);
    return `data:${contentType};base64,${b64}`;
  }, [isImage, rawBytes, contentType]);

  // Derive a file extension from content type for downloads.
  const fileExtension = useMemo(() => {
    if (!contentType) return "bin";
    const ct = contentType.toLowerCase().split(";")[0].trim();
    const map: Record<string, string> = {
      "image/png": "png",
      "image/jpeg": "jpg",
      "image/gif": "gif",
      "image/webp": "webp",
      "image/svg+xml": "svg",
      "image/x-icon": "ico",
      "image/bmp": "bmp",
      "image/avif": "avif",
      "image/tiff": "tiff",
    };
    return map[ct] || "bin";
  }, [contentType]);

  const handleDownload = useCallback(() => {
    if (!rawBytes) return;
    const blob = new Blob([rawBytes.buffer as ArrayBuffer], {
      type: contentType || "application/octet-stream",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `response.${fileExtension}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [rawBytes, contentType, fileExtension]);

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

  // Apply JSONPath or JMESPath query to the parsed JSON.
  // Returns { result, error } where result is the queried data (or null
  // if no query is active / query failed), and error is an error message.
  const queryResult = useMemo(() => {
    if (!parsedJson || queryMode === "none" || !queryString.trim()) {
      return { result: null as unknown, error: undefined as string | undefined };
    }
    try {
      if (queryMode === "jsonpath") {
        const res = JSONPath({ path: queryString, json: parsedJson });
        // JSONPath always returns an array; unwrap single-element arrays
        // for a cleaner display.
        const unwrapped = Array.isArray(res) && res.length === 1 ? res[0] : res;
        return { result: unwrapped, error: undefined };
      }
      // jmespath
      const res = jmespath.search(parsedJson, queryString);
      return { result: res, error: undefined };
    } catch (e) {
      return { result: null, error: e instanceof Error ? e.message : String(e) };
    }
  }, [parsedJson, queryMode, queryString]);

  // The effective JSON data to display: query result if a query is active,
  // otherwise the full parsed JSON.
  const effectiveJson = useMemo(() => {
    if (queryMode !== "none" && queryString.trim() && parsedJson) {
      return queryResult.error ? null : queryResult.result;
    }
    return parsedJson;
  }, [queryMode, queryString, parsedJson, queryResult]);

  const displayBody = useMemo(() => {
    if (!decodedBody) return "";

    // If query produced an error, show it
    if (isJson && queryResult.error) {
      return `Query error: ${queryResult.error}`;
    }
    return decodedBody;
  }, [isJson, decodedBody, queryResult]);

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
        {isCompressed && (
          <Button
            variant={showDecompressed ? "default" : "outline"}
            size="sm"
            onClick={() => setShowDecompressed(!showDecompressed)}
            title={
              showDecompressed
                ? "Show raw compressed data"
                : "Decompress body"
            }
          >
            <FileArchive className="h-4 w-4 mr-1" />
            {showDecompressed
              ? `Decompressed (${contentEncoding})`
              : `Raw (${contentEncoding})`}
          </Button>
        )}
        {isZstd && (
          <span className="text-xs font-mono px-2 py-0.5 rounded bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200">
            zstd
          </span>
        )}
        {isImage && imageDataUrl && (
          <Button
            variant={showAsImage ? "default" : "outline"}
            size="sm"
            onClick={() => setShowAsImage(!showAsImage)}
            title={showAsImage ? "Show raw data" : "Show as image"}
          >
            {showAsImage ? (
              <>
                <EyeOff className="h-4 w-4 mr-1" />
                Hide Image
              </>
            ) : (
              <>
                <Eye className="h-4 w-4 mr-1" />
                Show Image
              </>
            )}
          </Button>
        )}
        {isImage && rawBytes && (
          <Button
            variant="outline"
            size="sm"
            onClick={handleDownload}
            title="Download image"
          >
            <Download className="h-4 w-4 mr-1" />
            Download
          </Button>
        )}
        {isJson && parsedJson && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant={queryMode !== "none" ? "default" : "outline"}
                size="sm"
                title="JSON query tools"
              >
                <Terminal className="h-4 w-4 mr-1" />
                {queryMode === "none"
                  ? "JSON Tools"
                  : queryMode === "jsonpath"
                    ? "JSONPath"
                    : "JMESPath"}
                <ChevronDown className="h-3 w-3 ml-1" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuItem onClick={() => setQueryMode("none")}>
                None
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => {
                  setQueryMode("jsonpath");
                  setQueryString("");
                }}
              >
                JSONPath
                <span className="text-xs text-muted-foreground ml-2">
                  $.store.book[*].title
                </span>
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => {
                  setQueryMode("jmespath");
                  setQueryString("");
                }}
              >
                JMESPath
                <span className="text-xs text-muted-foreground ml-2">
                  store.book[*].title
                </span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
        {!isImage && !isJson && (
          <div className="relative flex-1 max-w-xs">
            <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search body..."
              className="pl-8 h-8"
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
            />
          </div>
        )}
        {!isImage && !isJson && (
          <Button variant="ghost" size="sm" onClick={handleCopy}>
            <Copy className="h-4 w-4 mr-1" />
            Copy
          </Button>
        )}
      </div>

      {/* JSON Query Input */}
      {isJson && parsedJson && queryMode !== "none" && (
        <div className="flex items-center gap-2">
          <code className="text-xs text-muted-foreground font-mono shrink-0">
            {queryMode === "jsonpath" ? "$" : "@"}
          </code>
          <Input
            placeholder={
              queryMode === "jsonpath"
                ? "$.store.book[*].title"
                : "store.book[*].title"
            }
            className="h-8 font-mono text-sm"
            value={queryString}
            onChange={(e) => setQueryString(e.target.value)}
          />
          {queryResult.error && (
            <span className="text-xs text-destructive shrink-0 max-w-xs truncate">
              {queryResult.error}
            </span>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              setQueryMode("none");
              setQueryString("");
            }}
            title="Clear query"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      )}

      {/* Body Content */}
      {isImage && showAsImage && imageDataUrl ? (
        <div className="space-y-2">
          <div className="border rounded-md p-4 bg-muted/30 flex items-center justify-center min-h-[100px]">
            <img
              src={imageDataUrl}
              alt="Response body"
              className="max-w-full max-h-[600px] object-contain rounded"
              onError={(e) => {
                const target = e.currentTarget;
                target.style.display = "none";
                const parent = target.parentElement;
                if (parent) {
                  parent.innerHTML =
                    '<p class="text-muted-foreground text-sm">Failed to render image. Try downloading instead.</p>';
                }
              }}
            />
          </div>
          {rawBytes && (
            <p className="text-xs text-muted-foreground">
              {contentType} — {formatBytes(rawBytes.length)}
            </p>
          )}
        </div>
      ) : isJson && effectiveJson != null ? (
        <JsonView data={effectiveJson} />
      ) : isJson && queryResult.error ? (
        <pre className="font-mono text-sm bg-destructive/10 text-destructive p-3 rounded-md overflow-x-auto whitespace-pre-wrap break-all">
          {displayBody}
        </pre>
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
            httpVersion: entry.request.http_version || "HTTP/1.1",
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
                httpVersion: entry.response.http_version || "HTTP/1.1",
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
