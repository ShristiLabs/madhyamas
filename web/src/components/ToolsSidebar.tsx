import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { BreakpointsPanel } from './BreakpointsPanel'
import { MocksPanel } from './MocksPanel'
import { RewritesPanel } from './RewritesPanel'
import { ThrottlePanel } from './ThrottlePanel'
import { ReplayPanel } from './ReplayPanel'
import { GrpcPanel } from './GrpcPanel'
import { ScriptsPanel } from './ScriptsPanel'
import { PluginsPanel } from './PluginsPanel'
import type { TrafficEntry } from '@/lib/api'
import {
  Pause,
  Theater,
  Pencil,
  Gauge,
  RotateCcw,
  Zap,
  Code,
  Puzzle,
  X,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react'
import { cn } from '@/lib/utils'

interface ToolsSidebarProps {
  selectedEntry?: TrafficEntry | null
  isOpen: boolean
  onClose: () => void
}

type TabValue = 'mocks' | 'rewrites' | 'breakpoints' | 'throttle' | 'replay' | 'grpc' | 'scripts' | 'plugins'

interface TabConfig {
  value: TabValue
  label: string
  icon: React.ComponentType<{ className?: string }>
  category: 'intercept' | 'modify' | 'extend' | 'debug'
}

const tabs: TabConfig[] = [
  { value: 'breakpoints', label: 'Breakpoints', icon: Pause, category: 'intercept' },
  { value: 'throttle', label: 'Throttle', icon: Gauge, category: 'intercept' },
  { value: 'mocks', label: 'Mocks', icon: Theater, category: 'modify' },
  { value: 'rewrites', label: 'Rewrites', icon: Pencil, category: 'modify' },
  { value: 'replay', label: 'Replay', icon: RotateCcw, category: 'debug' },
  { value: 'grpc', label: 'gRPC', icon: Zap, category: 'debug' },
  { value: 'scripts', label: 'Scripts', icon: Code, category: 'extend' },
  { value: 'plugins', label: 'Plugins', icon: Puzzle, category: 'extend' },
]

const categoryLabels: Record<string, string> = {
  intercept: 'Intercept',
  modify: 'Modify',
  debug: 'Debug',
  extend: 'Extend',
}

export function ToolsSidebar({ selectedEntry, isOpen, onClose }: ToolsSidebarProps) {
  const [activeTab, setActiveTab] = useState<TabValue>('mocks')
  const [isCollapsed, setIsCollapsed] = useState(false)

  if (!isOpen) return null

  const tabsByCategory = tabs.reduce((acc, tab) => {
    if (!acc[tab.category]) acc[tab.category] = []
    acc[tab.category].push(tab)
    return acc
  }, {} as Record<string, TabConfig[]>)

  return (
    <div className="h-full flex bg-background">
      <TooltipProvider delayDuration={0}>
        {/* Vertical Tab Bar */}
        <div className={cn(
          "flex flex-col border-r bg-muted/30 transition-all duration-200",
          isCollapsed ? "w-12" : "w-48"
        )}>
          {/* Header */}
          <div className="flex items-center justify-between p-2 border-b h-10">
            {!isCollapsed && (
              <span className="text-sm font-medium px-2">Tools</span>
            )}
            <Button
              variant="ghost"
              size="sm"
              className="h-6 w-6 p-0"
              onClick={onClose}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>

          {/* Tab Categories */}
          <div className="flex-1 overflow-y-auto py-2">
            {Object.entries(tabsByCategory).map(([category, categoryTabs]) => (
              <div key={category} className="mb-4">
                {!isCollapsed && (
                  <div className="px-3 py-1 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                    {categoryLabels[category]}
                  </div>
                )}
                <div className="space-y-1 px-2">
                  {categoryTabs.map((tab) => {
                    const Icon = tab.icon
                    const isActive = activeTab === tab.value

                    return (
                      <Tooltip key={tab.value}>
                        <TooltipTrigger asChild>
                          <button
                            onClick={() => setActiveTab(tab.value)}
                            className={cn(
                              "w-full flex items-center gap-3 px-2 py-2 rounded-md text-sm transition-colors",
                              isActive
                                ? "bg-primary text-primary-foreground"
                                : "hover:bg-muted text-foreground",
                              isCollapsed && "justify-center"
                            )}
                          >
                            <Icon className="h-4 w-4 flex-shrink-0" />
                            {!isCollapsed && <span>{tab.label}</span>}
                          </button>
                        </TooltipTrigger>
                        {isCollapsed && (
                          <TooltipContent side="right">
                            {tab.label}
                          </TooltipContent>
                        )}
                      </Tooltip>
                    )
                  })}
                </div>
              </div>
            ))}
          </div>

          {/* Collapse Button */}
          <div className="border-t p-2">
            <Button
              variant="ghost"
              size="sm"
              className="w-full"
              onClick={() => setIsCollapsed(!isCollapsed)}
            >
              {isCollapsed ? (
                <ChevronRight className="h-4 w-4" />
              ) : (
                <>
                  <ChevronLeft className="h-4 w-4 mr-2" />
                  <span className="text-xs">Collapse</span>
                </>
              )}
            </Button>
          </div>
        </div>

        {/* Panel Content */}
        <div className={cn(
          "flex-1 flex flex-col min-w-0 overflow-hidden",
          isCollapsed && "hidden"
        )}>
          <div className="flex-1 overflow-hidden">
            {activeTab === 'breakpoints' && <BreakpointsPanel />}
            {activeTab === 'mocks' && <MocksPanel />}
            {activeTab === 'rewrites' && <RewritesPanel />}
            {activeTab === 'throttle' && <ThrottlePanel />}
            {activeTab === 'replay' && <ReplayPanel selectedEntry={selectedEntry} />}
            {activeTab === 'grpc' && <GrpcPanel />}
            {activeTab === 'scripts' && <ScriptsPanel />}
            {activeTab === 'plugins' && <PluginsPanel />}
          </div>
        </div>
      </TooltipProvider>
    </div>
  )
}
