import type { TrafficEntry } from './traffic'

export type FilterCategory = 'request' | 'response'

export type FilterOperator =
  | 'eq' | 'neq' | 'contains' | 'not_contains'
  | 'starts_with' | 'ends_with' | 'regex'
  | 'gt' | 'lt' | 'gte' | 'lte'
  | 'exists' | 'not_exists'

export interface FilterFieldDef {
  id: string
  label: string
  category: FilterCategory
  valueType: 'text' | 'number' | 'select'
  operators: FilterOperator[]
  options?: { value: string; label: string }[]
  placeholder?: string
  hasKey?: boolean
}

export interface ActiveFilter {
  id: string
  fieldId: string
  operator: FilterOperator
  value: string
  key?: string
}

export const OPERATOR_LABELS: Record<FilterOperator, string> = {
  eq: '=',
  neq: '≠',
  contains: 'contains',
  not_contains: '!contains',
  starts_with: 'starts with',
  ends_with: 'ends with',
  regex: '~=',
  gt: '>',
  lt: '<',
  gte: '>=',
  lte: '<=',
  exists: 'exists',
  not_exists: '!exists',
}

const TEXT_OPS: FilterOperator[] = ['contains', 'eq', 'neq', 'not_contains', 'starts_with', 'ends_with', 'regex']
const NUM_OPS: FilterOperator[] = ['eq', 'gt', 'lt', 'gte', 'lte', 'neq']
const HEADER_OPS: FilterOperator[] = ['exists', 'not_exists', 'contains', 'eq', 'neq']

export const FILTER_FIELDS: FilterFieldDef[] = [
  {
    id: 'method', label: 'Method', category: 'request', valueType: 'select',
    operators: ['eq', 'neq'],
    options: ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'].map(m => ({ value: m, label: m })),
  },
  {
    id: 'protocol', label: 'Protocol', category: 'request', valueType: 'select',
    operators: ['eq', 'neq'],
    options: [{ value: 'https', label: 'HTTPS' }, { value: 'http', label: 'HTTP' }],
  },
  { id: 'domain', label: 'Domain', category: 'request', valueType: 'text', operators: TEXT_OPS, placeholder: 'example.com' },
  { id: 'path', label: 'Path', category: 'request', valueType: 'text', operators: TEXT_OPS, placeholder: '/api/users' },
  { id: 'url', label: 'URL', category: 'request', valueType: 'text', operators: TEXT_OPS, placeholder: 'https://...' },
  {
    id: 'file_type', label: 'File Extension', category: 'request', valueType: 'select',
    operators: ['eq', 'neq'],
    options: ['.js', '.css', '.png', '.jpg', '.svg', '.json', '.xml', '.html', '.woff', '.ttf', '.gif', '.ico', '.map', '.wasm']
      .map(e => ({ value: e, label: e })),
  },
  { id: 'request_header', label: 'Request Header', category: 'request', valueType: 'text', operators: HEADER_OPS, hasKey: true, placeholder: 'header value' },
  { id: 'request_content_type', label: 'Request Content-Type', category: 'request', valueType: 'text', operators: TEXT_OPS, placeholder: 'application/json' },
  { id: 'cookie', label: 'Cookie', category: 'request', valueType: 'text', operators: HEADER_OPS, hasKey: true, placeholder: 'cookie value' },
  { id: 'status_code', label: 'Status Code', category: 'response', valueType: 'number', operators: NUM_OPS, placeholder: '200' },
  {
    id: 'status_category', label: 'Status Category', category: 'response', valueType: 'select',
    operators: ['eq', 'neq'],
    options: [
      { value: '2xx', label: '2xx Success' },
      { value: '3xx', label: '3xx Redirect' },
      { value: '4xx', label: '4xx Client Error' },
      { value: '5xx', label: '5xx Server Error' },
    ],
  },
  { id: 'response_header', label: 'Response Header', category: 'response', valueType: 'text', operators: HEADER_OPS, hasKey: true, placeholder: 'header value' },
  { id: 'response_content_type', label: 'Response Content-Type', category: 'response', valueType: 'text', operators: TEXT_OPS, placeholder: 'text/html' },
  { id: 'duration', label: 'Duration (ms)', category: 'response', valueType: 'number', operators: NUM_OPS, placeholder: '1000' },
  { id: 'response_size', label: 'Response Size (bytes)', category: 'response', valueType: 'number', operators: NUM_OPS, placeholder: '10000' },
  {
    id: 'is_passthrough', label: 'SSL Passthrough', category: 'request', valueType: 'select',
    operators: ['eq', 'neq'],
    options: [
      { value: 'true', label: 'Passthrough' },
      { value: 'false', label: 'Intercepted' },
    ],
  },
]

function getFieldValue(entry: TrafficEntry, filter: ActiveFilter): string | number | undefined {
  switch (filter.fieldId) {
    case 'method':
      return entry.request.method
    case 'protocol':
      return entry.request.url.startsWith('https://') ? 'https' : 'http'
    case 'domain':
      return entry.request.host
    case 'path':
      return entry.request.path
    case 'url':
      return entry.request.url
    case 'file_type': {
      const p = entry.request.path.split('?')[0]
      const dot = p.lastIndexOf('.')
      return dot >= 0 ? p.substring(dot) : ''
    }
    case 'request_header': {
      const headers = entry.request.headers
      if (!filter.key) return Object.entries(headers).map(([k, v]) => `${k}: ${v}`).join('\n')
      const found = Object.keys(headers).find(k => k.toLowerCase() === filter.key!.toLowerCase())
      return found ? headers[found] : undefined
    }
    case 'request_content_type':
      return entry.request.content_type
    case 'cookie': {
      const cookieHeader = Object.entries(entry.request.headers)
        .find(([k]) => k.toLowerCase() === 'cookie')?.[1] ?? ''
      if (!filter.key) return cookieHeader
      const cookies = cookieHeader.split(';').map(c => c.trim())
      const found = cookies.find(c => c.toLowerCase().startsWith(filter.key!.toLowerCase() + '='))
      return found ? found.split('=').slice(1).join('=') : undefined
    }
    case 'status_code':
      return entry.response?.status_code
    case 'status_category': {
      if (!entry.response) return undefined
      return `${Math.floor(entry.response.status_code / 100)}xx`
    }
    case 'response_header': {
      if (!entry.response) return undefined
      const headers = entry.response.headers
      if (!filter.key) return Object.entries(headers).map(([k, v]) => `${k}: ${v}`).join('\n')
      const found = Object.keys(headers).find(k => k.toLowerCase() === filter.key!.toLowerCase())
      return found ? headers[found] : undefined
    }
    case 'response_content_type':
      return entry.response?.content_type
    case 'duration':
      return entry.response?.duration_ms
    case 'response_size':
      return entry.response?.body?.length ?? 0
    case 'is_passthrough':
      return entry.is_passthrough ? 'true' : 'false'
    default:
      return undefined
  }
}

export function matchesFilter(entry: TrafficEntry, filter: ActiveFilter): boolean {
  const field = FILTER_FIELDS.find(f => f.id === filter.fieldId)
  if (!field) return true

  const value = getFieldValue(entry, filter)

  if (filter.operator === 'exists') return value !== undefined && value !== null && value !== ''
  if (filter.operator === 'not_exists') return value === undefined || value === null || value === ''

  if (value === undefined || value === null) return false

  const strValue = String(value).toLowerCase()
  const filterValue = filter.value.toLowerCase()

  switch (filter.operator) {
    case 'eq':
      return field.valueType === 'number'
        ? Number(value) === Number(filter.value)
        : strValue === filterValue
    case 'neq':
      return field.valueType === 'number'
        ? Number(value) !== Number(filter.value)
        : strValue !== filterValue
    case 'contains':
      return strValue.includes(filterValue)
    case 'not_contains':
      return !strValue.includes(filterValue)
    case 'starts_with':
      return strValue.startsWith(filterValue)
    case 'ends_with':
      return strValue.endsWith(filterValue)
    case 'regex':
      try { return new RegExp(filter.value, 'i').test(String(value)) }
      catch { return false }
    case 'gt':
      return Number(value) > Number(filter.value)
    case 'lt':
      return Number(value) < Number(filter.value)
    case 'gte':
      return Number(value) >= Number(filter.value)
    case 'lte':
      return Number(value) <= Number(filter.value)
    default:
      return true
  }
}

export function applyFilters(entries: TrafficEntry[], filters: ActiveFilter[]): TrafficEntry[] {
  if (filters.length === 0) return entries
  return entries.filter(entry => filters.every(f => matchesFilter(entry, f)))
}
