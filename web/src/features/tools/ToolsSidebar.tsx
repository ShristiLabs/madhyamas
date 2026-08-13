import { useState, lazy, Suspense } from 'react'
import { Button } from '@/components/ui/button'
import { BreakpointsPanel } from '@/features/tools/BreakpointsPanel'
import { ThrottlePanel } from '@/features/tools/ThrottlePanel'
import { RewritesPanel } from '@/features/tools/RewritesPanel'

// Lazy-load the heavier panels to keep the initial bundle small.
const MocksPanel = lazy(() => import('@/features/tools/MocksPanel').then((m) => ({ default: m.MocksPanel })))
const ReplayPanel = lazy(() => import('@/features/tools/ReplayPanel').then((m) => ({ default: m.ReplayPanel })))
const GrpcPanel = lazy(() => import('@/features/tools/GrpcPanel').then((m) => ({ default: m.GrpcPanel })))
const ScriptsPanel = lazy(() => import('@/features/tools/ScriptsPanel').then((m) => ({ default: m.ScriptsPanel })))
const PluginsPanel = lazy(() => import('@/features/tools/PluginsPanel').then((m) => ({ default: m.PluginsPanel })))
const BlockListPanel = lazy(() => import('@/features/tools/BlockListPanel').then((m) => ({ default: m.BlockListPanel })))
const MirrorPanel = lazy(() => import('@/features/tools/MirrorPanel').then((m) => ({ default: m.MirrorPanel })))
import {
  Pause,
  Theater,
  Pencil,
  Gauge,
  RotateCcw,
  Zap,
  Code,
  Puzzle,
  Shield,
  HardDriveDownload,
  X,
} from 'lucide-react'
import { cn } from '@/lib/utils'

interface ToolsSidebarProps {
  isOpen: boolean
  onClose: () => void
}

type TabValue = 'mocks' | 'rewrites' | 'breakpoints' | 'throttle' | 'replay' | 'grpc' | 'scripts' | 'plugins' | 'blocklist' | 'mirror'

interface TabConfig {
  value: TabValue
  label: string
  icon: React.ComponentType<{ className?: string }>
  category: 'intercept' | 'modify' | 'extend' | 'debug'
}

const tabs: TabConfig[] = [
  { value: 'breakpoints', label: 'Breakpoints', icon: Pause, category: 'intercept' },
  { value: 'blocklist', label: 'Block List', icon: Shield, category: 'intercept' },
  { value: 'throttle', label: 'Throttle', icon: Gauge, category: 'intercept' },
  { value: 'mocks', label: 'Mocks', icon: Theater, category: 'modify' },
  { value: 'rewrites', label: 'Rewrites', icon: Pencil, category: 'modify' },
  { value: 'replay', label: 'Replay', icon: RotateCcw, category: 'debug' },
  { value: 'mirror', label: 'Mirror', icon: HardDriveDownload, category: 'debug' },
  { value: 'grpc', label: 'gRPC', icon: Zap, category: 'debug' },
  { value: 'scripts', label: 'Scripts', icon: Code, category: 'extend' },
  { value: 'plugins', label: 'Plugins', icon: Puzzle, category: 'extend' },
]

const categoryOrder: TabConfig['category'][] = ['intercept', 'modify', 'debug', 'extend']
const categoryLabels: Record<string, string> = {
  intercept: 'Intercept',
  modify: 'Modify',
  debug: 'Debug',
  extend: 'Extend',
}

export function ToolsSidebar({ isOpen, onClose }: ToolsSidebarProps) {
  const [activeTab, setActiveTab] = useState<TabValue>('mocks')

  if (!isOpen) return null

  const tabsByCategory = categoryOrder.map((category) => ({
    category,
    items: tabs.filter((t) => t.category === category),
  }))

  return (
    <div className="flex h-full flex-col bg-background">
      {/* Header bar */}
      <div className="flex shrink-0 items-center justify-between border-b border-border px-2.5 py-1.5">
        <span className="text-2xs font-semibold uppercase tracking-wider text-muted-foreground">
          Tools
        </span>
        <Button variant="ghost" size="icon-sm" onClick={onClose} title="Close tools (T)">
          <X className="h-3.5 w-3.5" />
        </Button>
      </div>

      {/* Grouped horizontal tab bar */}
      <div className="shrink-0 space-y-1 border-b border-border bg-muted/20 px-2 py-1.5">
        {tabsByCategory.map(({ category, items }) => (
          <div key={category} className="flex items-center gap-1.5">
            <span className="w-14 shrink-0 text-2xs font-medium uppercase tracking-wider text-muted-foreground/70">
              {categoryLabels[category]}
            </span>
            <div className="flex flex-1 flex-wrap gap-1">
              {items.map((tab) => {
                const Icon = tab.icon
                const isActive = activeTab === tab.value
                return (
                  <button
                    key={tab.value}
                    onClick={() => setActiveTab(tab.value)}
                    className={cn(
                      'inline-flex items-center gap-1 rounded px-2 py-0.5 text-2xs font-medium transition-colors',
                      isActive
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-accent hover:text-foreground',
                    )}
                  >
                    <Icon className="h-3 w-3 shrink-0" />
                    {tab.label}
                  </button>
                )
              })}
            </div>
          </div>
        ))}
      </div>

      {/* Panel content — full width */}
      <div className="min-h-0 flex-1 overflow-hidden">
        <Suspense
          fallback={
            <div className="flex h-full items-center justify-center text-2xs text-muted-foreground">
              Loading…
            </div>
          }
        >
          {activeTab === 'breakpoints' && <BreakpointsPanel />}
          {activeTab === 'blocklist' && <BlockListPanel />}
          {activeTab === 'mocks' && <MocksPanel />}
          {activeTab === 'rewrites' && <RewritesPanel />}
          {activeTab === 'throttle' && <ThrottlePanel />}
          {activeTab === 'replay' && <ReplayPanel />}
          {activeTab === 'mirror' && <MirrorPanel />}
          {activeTab === 'grpc' && <GrpcPanel />}
          {activeTab === 'scripts' && <ScriptsPanel />}
          {activeTab === 'plugins' && <PluginsPanel />}
        </Suspense>
      </div>
    </div>
  )
}
