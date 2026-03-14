interface HeadersViewProps {
  headers: Record<string, string>
}

export function HeadersView({ headers }: HeadersViewProps) {
  const entries = Object.entries(headers)

  if (entries.length === 0) {
    return <div className="text-muted-foreground">No headers</div>
  }

  return (
    <div className="font-mono text-sm space-y-1">
      {entries.map(([key, value]) => (
        <div key={key} className="flex">
          <span className="font-semibold text-primary min-w-32">{key}:</span>
          <span className="text-muted-foreground break-all">{value}</span>
        </div>
      ))}
    </div>
  )
}
