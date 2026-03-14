import { useMemo } from 'react'
import { JsonView as ReactJsonView, darkStyles } from 'react-json-view-lite'
import 'react-json-view-lite/dist/index.css'

interface JsonViewProps {
  data: unknown
  searchTerm?: string
}

export function JsonView({ data, searchTerm }: JsonViewProps) {
  // Cast to acceptable type for the library
  const jsonData = data as Record<string, unknown> | unknown[]

  // If there's a search term, we'll highlight matching text in the rendered output
  const shouldHighlight = Boolean(searchTerm && searchTerm.length > 0)

  // Custom styles with highlighting support
  const customStyles = {
    ...darkStyles,
    container: 'bg-muted rounded-md p-3',
    label: 'text-primary font-semibold',
    stringValue: 'text-green-600 dark:text-green-400',
    numberValue: 'text-blue-600 dark:text-blue-400',
    booleanValue: 'text-purple-600 dark:text-purple-400',
    nullValue: 'text-red-600 dark:text-red-400',
  }

  // If search term exists, we need to check if the data contains it
  const hasMatch = useMemo(() => {
    if (!searchTerm) return true
    const searchLower = searchTerm.toLowerCase()
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
  }, [data, searchTerm])

  // If no match, show a message
  if (shouldHighlight && !hasMatch) {
    return (
      <div className="text-muted-foreground text-sm italic p-3">
        No matches found for "{searchTerm}"
      </div>
    )
  }

  return (
    <div className="font-mono text-sm" data-search={searchTerm || undefined}>
      <ReactJsonView data={jsonData} style={customStyles} />
    </div>
  )
}
