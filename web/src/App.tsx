import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { TrafficView } from './components/TrafficView'
import { CertificateHelper } from './components/CertificateHelper'
import { Toaster } from './components/ui/toaster'
import { Button } from './components/ui/button'
import { Download, Moon, Sun } from 'lucide-react'
import { useState, useEffect } from 'react'
import { useToast } from './components/ui/use-toast'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      staleTime: 1000,
    },
  },
})

function App() {
  const [isDark, setIsDark] = useState(() => {
    if (typeof window !== 'undefined') {
      return document.documentElement.classList.contains('dark')
    }
    return false
  })
  const { toast } = useToast()

  useEffect(() => {
    if (isDark) {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
  }, [isDark])

  const handleExportHar = async () => {
    try {
      const response = await fetch('/api/export/har')
      if (!response.ok) throw new Error('Failed to export HAR')

      const har = await response.json()
      const blob = new Blob([JSON.stringify(har, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `proxyforge-${new Date().toISOString().slice(0, 10)}.har`
      a.click()
      URL.revokeObjectURL(url)

      toast({ description: 'HAR file exported successfully' })
    } catch (error) {
      toast({ description: 'Failed to export HAR file', variant: 'destructive' })
    }
  }

  return (
    <QueryClientProvider client={queryClient}>
      <div className="h-screen flex flex-col bg-background">
        <header className="border-b px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-primary flex items-center justify-center">
              <span className="text-primary-foreground font-bold text-sm">PF</span>
            </div>
            <h1 className="text-xl font-semibold">ProxyForge</h1>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground mr-2">
              Proxy: localhost:8888
            </span>
            <CertificateHelper />
            <Button variant="ghost" size="sm" onClick={handleExportHar}>
              <Download className="h-4 w-4 mr-1" />
              Export HAR
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setIsDark(!isDark)}
            >
              {isDark ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </Button>
          </div>
        </header>
        <main className="flex-1 overflow-hidden">
          <TrafficView />
        </main>
      </div>
      <Toaster />
    </QueryClientProvider>
  )
}

export default App
