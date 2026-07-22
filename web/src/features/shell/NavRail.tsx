import {
  Activity,
  Pause,
  Gauge,
  Theater,
  Pencil,
  RotateCcw,
  Zap,
  Code,
  Puzzle,
  FolderTree,
  type LucideIcon,
} from "lucide-react"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"

export interface NavView {
  id: string
  label: string
  icon: string
  /** Mark as experimental — shows a small "beta" indicator in the tooltip */
  experimental?: boolean
}

const ICONS: Record<string, LucideIcon> = {
  Activity,
  Pause,
  Gauge,
  Theater,
  Pencil,
  RotateCcw,
  Zap,
  Code,
  Puzzle,
  FolderTree,
}

interface NavRailProps {
  views: NavView[]
  activeView: string
  onSelect: (id: NavView["id"]) => void
}

export function NavRail({ views, activeView, onSelect }: NavRailProps) {
  return (
    <TooltipProvider delayDuration={200}>
      <nav className="flex w-12 shrink-0 flex-col items-center gap-0.5 border-r border-border bg-sidebar py-2">
        {views.map((view) => {
          const Icon = ICONS[view.icon] ?? Activity
          const isActive = activeView === view.id
          return (
            <Tooltip key={view.id}>
              <TooltipTrigger asChild>
                <button
                  onClick={() => onSelect(view.id)}
                  className={cn(
                    "flex h-9 w-9 items-center justify-center rounded-md transition-colors",
                    isActive
                      ? "bg-primary text-primary-foreground"
                      : "text-sidebar-foreground hover:bg-accent hover:text-foreground",
                  )}
                  aria-label={view.label}
                  aria-current={isActive ? "page" : undefined}
                >
                  <Icon className="h-4 w-4" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right" sideOffset={8}>
                <span className="flex items-center gap-1.5">
                  {view.label}
                  {view.experimental && (
                    <span className="rounded bg-amber-500/20 px-1 py-0.5 text-[10px] font-medium text-amber-600 dark:text-amber-400">
                      Beta
                    </span>
                  )}
                </span>
              </TooltipContent>
            </Tooltip>
          )
        })}
      </nav>
    </TooltipProvider>
  )
}
