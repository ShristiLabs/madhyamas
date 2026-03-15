export interface RequestData {
  method: HttpMethod
  url: string
  host: string
  path: string
  headers: Record<string, string>
  body?: string
  content_type?: string
}

export interface ResponseData {
  status_code: number
  status_message?: string
  headers: Record<string, string>
  body?: string
  content_type?: string
  duration_ms: number
}

export interface TrafficEntry {
  id: string
  session_id: string
  request: RequestData
  response?: ResponseData
  timestamp: string
  modified: boolean
  notes?: string
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
