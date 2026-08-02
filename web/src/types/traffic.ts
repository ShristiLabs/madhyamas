export interface RequestData {
  method: HttpMethod
  url: string
  host: string
  path: string
  headers: Record<string, string>
  body?: string
  content_type?: string
  /** HTTP protocol version negotiated with the proxy (e.g. "HTTP/1.1", "HTTP/2"). */
  http_version?: string
}

export interface ResponseData {
  status_code: number
  status_message?: string
  headers: Record<string, string>
  body?: string
  content_type?: string
  duration_ms: number
  /** HTTP protocol version of the response (e.g. "HTTP/1.1", "HTTP/2"). */
  http_version?: string
}

export interface TrafficEntry {
  id: string
  session_id: string
  request: RequestData
  response?: ResponseData
  timestamp: string
  modified: boolean
  notes?: string
  /** Total request size in bytes (headers + body). */
  request_size?: number
  /** Total response size in bytes (headers + body), if a response was received. */
  response_size?: number
  /** Whether this connection was SSL-passed-through (not intercepted). */
  is_passthrough?: boolean
  /** Whether at least one script ran on this request (on_request or
   *  on_response hook).  Set by the proxy pipeline. */
  script_intercepted?: boolean
}

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE' | 'HEAD' | 'OPTIONS' | 'CONNECT' | 'TRACE'

export interface TrafficFilter {
  url?: string
  method?: HttpMethod
  statusCode?: string
  limit?: number
  offset?: number
  search?: string
  minDuration?: number
  maxDuration?: number
  host?: string
  contentType?: string
  fileType?: string
  header?: string
  cookie?: string
}

export interface Session {
  id: string
  name?: string
  created_at: string
  updated_at: string
}

export interface FocusHost {
  id: string
  pattern: string
  created_at: string
}
