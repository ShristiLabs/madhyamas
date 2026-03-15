import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { useToast } from '@/components/ui/use-toast'
import {
  Download,
  Smartphone,
  Apple,
  HelpCircle,
  CheckCircle,
  ChevronRight,
  Copy,
  Shield,
  Key,
} from 'lucide-react'

interface InstructionStep {
  title: string
  description: string
  subSteps?: string[]
}

const IOS_INSTRUCTIONS: InstructionStep[] = [
  {
    title: 'Download Certificate',
    description: 'Click the download button above to get the Madhyamas CA certificate.',
  },
  {
    title: 'Open Settings',
    description: 'On your iOS device, open Settings > General > VPN & Device Management.',
  },
  {
    title: 'Install Profile',
    description: 'Find the Madhyamas CA profile and tap Install. Enter your passcode if prompted.',
  },
  {
    title: 'Trust Certificate',
    description: 'Go to Settings > General > About > Certificate Trust Settings.',
    subSteps: [
      'Find Madhyamas CA in the list',
      'Toggle the switch to enable full trust',
      'You should see "Enabled" status',
    ],
  },
  {
    title: 'Configure Proxy',
    description: 'Go to Settings > Wi-Fi, tap your network, scroll to HTTP Proxy.',
    subSteps: [
      'Select "Manual"',
      'Server: Enter your computer\'s IP address',
      'Port: 8888',
      'Authentication: Off',
    ],
  },
]

const ANDROID_INSTRUCTIONS: InstructionStep[] = [
  {
    title: 'Download Certificate',
    description: 'Click the download button above to get the Madhyamas CA certificate.',
  },
  {
    title: 'Open Settings',
    description: 'On your Android device, go to Settings > Security (or Security & Location).',
  },
  {
    title: 'Install Certificate',
    description: 'Tap "Install from storage" or "Install certificates" > "CA certificate".',
    subSteps: [
      'You may need to confirm your PIN/password',
      'Navigate to your Downloads folder',
      'Select the madhyamas-ca.crt file',
      'Name it "Madhyamas CA" and tap OK',
    ],
  },
  {
    title: 'Configure Proxy',
    description: 'Go to Settings > Wi-Fi, long-press your network, select "Modify network".',
    subSteps: [
      'Show advanced options',
      'Proxy: Manual',
      'Hostname: Enter your computer\'s IP address',
      'Port: 8888',
      'Save',
    ],
  },
]

export function CertificatePanel() {
  const { toast } = useToast()
  const [showIOSDialog, setShowIOSDialog] = useState(false)
  const [showAndroidDialog, setShowAndroidDialog] = useState(false)

  const handleDownload = async () => {
    try {
      const response = await fetch('/api/cert/ca')
      if (!response.ok) {
        throw new Error('Failed to download certificate')
      }
      const blob = await response.blob()
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'madhyamas-ca.crt'
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)

      toast({
        title: 'Certificate Downloaded',
        description: 'Install this certificate on your devices to enable HTTPS interception.',
      })
    } catch (error) {
      toast({
        title: 'Download Failed',
        description: 'Could not download the certificate. Make sure HTTPS interception is enabled.',
        variant: 'destructive',
      })
    }
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
    toast({ description: 'Copied to clipboard' })
  }

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b">
        <h2 className="text-lg font-semibold">SSL Certificate</h2>
        <p className="text-sm text-muted-foreground">
          Install the Madhyamas CA certificate to intercept HTTPS traffic
        </p>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-4 space-y-6">
          {/* Status Card */}
          <div className="p-4 bg-muted/50 rounded-lg border">
            <div className="flex items-start gap-3">
              <div className="p-2 bg-green-100 dark:bg-green-900/30 rounded-lg">
                <Shield className="h-5 w-5 text-green-600 dark:text-green-400" />
              </div>
              <div className="flex-1">
                <h3 className="font-medium">Certificate Ready</h3>
                <p className="text-sm text-muted-foreground mt-1">
                  The Madhyamas CA certificate is available for download. Install it on your
                  devices to enable HTTPS traffic interception.
                </p>
              </div>
            </div>
          </div>

          {/* Download Button */}
          <div className="flex flex-col items-center py-6 border-2 border-dashed rounded-lg">
            <Key className="h-12 w-12 text-muted-foreground mb-4" />
            <Button onClick={handleDownload} size="lg" className="mb-2">
              <Download className="h-4 w-4 mr-2" />
              Download Certificate
            </Button>
            <p className="text-sm text-muted-foreground">madhyamas-ca.crt</p>
          </div>

          {/* Platform Instructions */}
          <div className="space-y-4">
            <h3 className="font-medium">Installation Instructions</h3>

            {/* iOS */}
            <div className="border rounded-lg overflow-hidden">
              <button
                className="w-full flex items-center justify-between p-4 hover:bg-muted/50 transition-colors"
                onClick={() => setShowIOSDialog(true)}
              >
                <div className="flex items-center gap-3">
                  <Apple className="h-5 w-5" />
                  <span className="font-medium">iOS (iPhone/iPad)</span>
                </div>
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              </button>
            </div>

            {/* Android */}
            <div className="border rounded-lg overflow-hidden">
              <button
                className="w-full flex items-center justify-between p-4 hover:bg-muted/50 transition-colors"
                onClick={() => setShowAndroidDialog(true)}
              >
                <div className="flex items-center gap-3">
                  <Smartphone className="h-5 w-5" />
                  <span className="font-medium">Android</span>
                </div>
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              </button>
            </div>
          </div>

          {/* Proxy Configuration */}
          <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
            <h4 className="font-medium text-blue-800 dark:text-blue-300 mb-2">
              Proxy Configuration
            </h4>
            <div className="space-y-2 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-blue-700 dark:text-blue-400">Host:</span>
                <div className="flex items-center gap-2">
                  <code className="px-2 py-1 bg-blue-100 dark:bg-blue-800 rounded text-blue-800 dark:text-blue-200">
                    Your computer's IP
                  </code>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 w-6 p-0"
                    onClick={() => {
                      // Try to detect local IP
                      const pc = new RTCPeerConnection({ iceServers: [] })
                      pc.createDataChannel('')
                      pc.createOffer().then(offer => pc.setLocalDescription(offer))
                      pc.onicecandidate = (ice) => {
                        if (ice && ice.candidate) {
                          const ip = ice.candidate.address
                          if (ip) {
                            copyToClipboard(ip)
                            pc.close()
                          }
                        }
                      }
                    }}
                  >
                    <Copy className="h-3 w-3" />
                  </Button>
                </div>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-blue-700 dark:text-blue-400">Port:</span>
                <code className="px-2 py-1 bg-blue-100 dark:bg-blue-800 rounded text-blue-800 dark:text-blue-200">
                  8888
                </code>
              </div>
            </div>
          </div>

          {/* Warning */}
          <div className="p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800">
            <div className="flex gap-2">
              <HelpCircle className="h-4 w-4 text-yellow-600 dark:text-yellow-400 flex-shrink-0 mt-0.5" />
              <div className="text-sm text-yellow-700 dark:text-yellow-300">
                <p className="font-medium">Important Notes:</p>
                <ul className="mt-1 space-y-1 list-disc list-inside">
                  <li>Only install this certificate on devices you control</li>
                  <li>Remove the certificate when done debugging</li>
                  <li>The certificate allows Madhyamas to decrypt HTTPS traffic</li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      </ScrollArea>

      {/* iOS Instructions Dialog */}
      <Dialog open={showIOSDialog} onOpenChange={setShowIOSDialog}>
        <DialogContent className="max-w-lg max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Apple className="h-5 w-5" />
              iOS Installation Instructions
            </DialogTitle>
            <DialogDescription>
              Follow these steps to install the Madhyamas CA certificate on your iOS device
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {IOS_INSTRUCTIONS.map((step, index) => (
              <div key={index} className="flex gap-3">
                <div className="flex-shrink-0 w-6 h-6 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-sm font-medium">
                  {index + 1}
                </div>
                <div className="flex-1">
                  <h4 className="font-medium">{step.title}</h4>
                  <p className="text-sm text-muted-foreground mt-0.5">{step.description}</p>
                  {step.subSteps && (
                    <ul className="mt-2 space-y-1">
                      {step.subSteps.map((subStep, subIndex) => (
                        <li key={subIndex} className="flex items-start gap-2 text-sm">
                          <CheckCircle className="h-4 w-4 text-green-500 flex-shrink-0 mt-0.5" />
                          <span>{subStep}</span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            ))}
          </div>
          <div className="flex justify-end">
            <Button onClick={() => setShowIOSDialog(false)}>Got it</Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Android Instructions Dialog */}
      <Dialog open={showAndroidDialog} onOpenChange={setShowAndroidDialog}>
        <DialogContent className="max-w-lg max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Smartphone className="h-5 w-5" />
              Android Installation Instructions
            </DialogTitle>
            <DialogDescription>
              Follow these steps to install the Madhyamas CA certificate on your Android device
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {ANDROID_INSTRUCTIONS.map((step, index) => (
              <div key={index} className="flex gap-3">
                <div className="flex-shrink-0 w-6 h-6 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-sm font-medium">
                  {index + 1}
                </div>
                <div className="flex-1">
                  <h4 className="font-medium">{step.title}</h4>
                  <p className="text-sm text-muted-foreground mt-0.5">{step.description}</p>
                  {step.subSteps && (
                    <ul className="mt-2 space-y-1">
                      {step.subSteps.map((subStep, subIndex) => (
                        <li key={subIndex} className="flex items-start gap-2 text-sm">
                          <CheckCircle className="h-4 w-4 text-green-500 flex-shrink-0 mt-0.5" />
                          <span>{subStep}</span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            ))}
          </div>
          <div className="flex justify-end">
            <Button onClick={() => setShowAndroidDialog(false)}>Got it</Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}
