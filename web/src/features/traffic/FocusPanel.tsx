import { useState, useCallback } from "react"
import { Star, X, Plus, Trash2, Eye, EyeOff } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Switch } from "@/components/ui/switch"
import {
  useFocusHosts,
  useAddFocusHost,
  useRemoveFocusHost,
  useClearFocusHosts,
} from "@/lib/api/intercept"

interface FocusPanelProps {
  showOnlyFocused: boolean
  onShowOnlyFocusedChange: (value: boolean) => void
}

export function FocusPanel({ showOnlyFocused, onShowOnlyFocusedChange }: FocusPanelProps) {
  const { data: focusHosts, isLoading } = useFocusHosts()
  const addFocusHost = useAddFocusHost()
  const removeFocusHost = useRemoveFocusHost()
  const clearFocusHosts = useClearFocusHosts()
  const [inputValue, setInputValue] = useState("")

  const handleAdd = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault()
      const pattern = inputValue.trim()
      if (!pattern) return
      addFocusHost.mutate(pattern, {
        onSuccess: () => setInputValue(""),
      })
    },
    [inputValue, addFocusHost],
  )

  const handleRemove = useCallback(
    (id: string) => {
      removeFocusHost.mutate(id)
    },
    [removeFocusHost],
  )

  const handleClearAll = useCallback(() => {
    if (focusHosts && focusHosts.length > 0) {
      clearFocusHosts.mutate()
    }
  }, [focusHosts, clearFocusHosts])

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div className="flex items-center gap-1.5">
          <Star className="h-3.5 w-3.5 text-amber-500" />
          <span className="text-xs font-semibold">Focus Hosts</span>
          {focusHosts && focusHosts.length > 0 && (
            <span className="rounded-full bg-muted px-1.5 py-px text-2xs text-muted-foreground">
              {focusHosts.length}
            </span>
          )}
        </div>
        {focusHosts && focusHosts.length > 0 && (
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleClearAll}
            title="Clear all focus hosts"
            disabled={clearFocusHosts.isPending}
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        )}
      </div>

      <div className="border-b border-border px-3 py-2">
        <form onSubmit={handleAdd} className="flex items-center gap-1.5">
          <Input
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            placeholder="e.g. *.example.com"
            className="h-7 text-2xs"
            disabled={addFocusHost.isPending}
          />
          <Button
            type="submit"
            variant="ghost"
            size="icon-sm"
            disabled={!inputValue.trim() || addFocusHost.isPending}
            title="Add focus host"
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
        </form>
      </div>

      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div className="flex items-center gap-1.5">
          {showOnlyFocused ? (
            <Eye className="h-3 w-3 text-amber-500" />
          ) : (
            <EyeOff className="h-3 w-3 text-muted-foreground" />
          )}
          <span className="text-2xs text-muted-foreground">Show only focused</span>
        </div>
        <Switch checked={showOnlyFocused} onCheckedChange={onShowOnlyFocusedChange} />
      </div>

      <div className="flex-1 overflow-auto">
        {isLoading ? (
          <div className="p-3 text-center text-2xs text-muted-foreground">Loading…</div>
        ) : !focusHosts || focusHosts.length === 0 ? (
          <div className="p-3 text-center text-2xs text-muted-foreground">
            No focus hosts yet.
            <br />
            Add a pattern above or right-click a traffic row.
          </div>
        ) : (
          <div className="py-1">
            {focusHosts.map((host) => (
              <div
                key={host.id}
                className="group flex items-center gap-1.5 px-3 py-1 hover:bg-muted/40"
              >
                <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-amber-500" />
                <span
                  className="min-w-0 flex-1 truncate font-mono text-2xs font-medium"
                  title={host.pattern}
                >
                  {host.pattern}
                </span>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="h-4 w-4 opacity-0 group-hover:opacity-100"
                  onClick={() => handleRemove(host.id)}
                  title="Remove"
                >
                  <X className="h-3 w-3" />
                </Button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
