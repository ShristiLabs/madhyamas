import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { CheckCircle, ChevronRight, ChevronLeft, Download, Lightbulb, X } from 'lucide-react'

interface OnboardingStep {
  id: string
  title: string
  description: string
  completed: boolean
  optional: boolean
}

interface OnboardingStatus {
  completed: boolean
  current_step: number
  total_steps: number
  steps: OnboardingStep[]
}

interface OnboardingWizardProps {
  isOpen: boolean
  onClose: () => void
}

export function OnboardingWizard({ isOpen, onClose }: OnboardingWizardProps) {
  const [status, setStatus] = useState<OnboardingStatus | null>(null)
  const [currentStep, setCurrentStep] = useState(0)

  useEffect(() => {
    if (isOpen) {
      fetch('/api/onboarding')
        .then(res => res.json())
        .then(setStatus)
        .catch(console.error)
    }
  }, [isOpen])

  const completeStep = async (stepId: string) => {
    await fetch('/api/onboarding/complete', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ step_id: stepId })
    })

    if (status) {
      setStatus({
        ...status,
        steps: status.steps.map(s =>
          s.id === stepId ? { ...s, completed: true } : s
        )
      })
    }
  }

  const skipOnboarding = async () => {
    await fetch('/api/onboarding/skip', { method: 'POST' })
    onClose()
  }

  if (!isOpen || !status) return null

  const step = status.steps[currentStep]
  const isLastStep = currentStep === status.steps.length - 1
  const isFirstStep = currentStep === 0

  const renderStepContent = (stepId: string) => {
    switch (stepId) {
      case 'welcome':
        return (
          <div className="space-y-4">
            <p className="text-muted-foreground">
              Welcome to Madhyamas! This wizard will help you get started with intercepting and debugging HTTP/HTTPS traffic.
            </p>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div className="p-4 bg-muted rounded-lg">
                <h4 className="font-medium mb-2">What you can do:</h4>
                <ul className="space-y-1 text-muted-foreground">
                  <li>• Capture HTTP/HTTPS traffic</li>
                  <li>• Set breakpoints to modify requests</li>
                  <li>• Mock responses for testing</li>
                  <li>• Throttle network speed</li>
                </ul>
              </div>
              <div className="p-4 bg-muted rounded-lg">
                <h4 className="font-medium mb-2">Supported protocols:</h4>
                <ul className="space-y-1 text-muted-foreground">
                  <li>• HTTP/1.1 and HTTP/2</li>
                  <li>• WebSocket</li>
                  <li>• gRPC</li>
                  <li>• TLS/SSL interception</li>
                </ul>
              </div>
            </div>
          </div>
        )

      case 'certificate':
        return (
          <div className="space-y-4">
            <p className="text-muted-foreground">
              To intercept HTTPS traffic, you need to install Madhyamas's root CA certificate.
            </p>
            <div className="p-4 bg-yellow-50 dark:bg-yellow-950 rounded-lg border border-yellow-200 dark:border-yellow-800">
              <h4 className="font-medium text-yellow-800 dark:text-yellow-200 mb-2">Security Note</h4>
              <p className="text-sm text-yellow-700 dark:text-yellow-300">
                The certificate is generated locally and never leaves your machine. It's only used to decrypt HTTPS traffic for debugging.
              </p>
            </div>
            <Button onClick={() => window.open('/api/cert/ca', '_blank')}>
              <Download className="h-4 w-4 mr-2" />
              Download Certificate
            </Button>
            <p className="text-xs text-muted-foreground">
              After downloading, double-click the certificate and add it to your system's trusted root certificates.
            </p>
          </div>
        )

      case 'proxy':
        return (
          <div className="space-y-4">
            <p className="text-muted-foreground">
              Configure your browser or application to use Madhyamas as a proxy.
            </p>
            <div className="p-4 bg-muted rounded-lg font-mono text-sm">
              <p><strong>Proxy Host:</strong> localhost</p>
              <p><strong>Proxy Port:</strong> 8888</p>
              <p><strong>Proxy Type:</strong> HTTP/HTTPS</p>
            </div>
            <div className="space-y-2">
              <h4 className="font-medium">Quick Setup:</h4>
              <ul className="text-sm text-muted-foreground space-y-1">
                <li>• <strong>Chrome/Edge:</strong> Settings → System → Open proxy settings</li>
                <li>• <strong>Firefox:</strong> Settings → Network Settings → Manual proxy</li>
                <li>• <strong>macOS:</strong> System Preferences → Network → Advanced → Proxies</li>
                <li>• <strong>Terminal:</strong> export HTTP_PROXY=http://localhost:8888</li>
              </ul>
            </div>
          </div>
        )

      case 'features':
        return (
          <div className="space-y-4">
            <p className="text-muted-foreground">
              Explore Madhyamas's powerful features for debugging and testing.
            </p>
            <div className="grid grid-cols-1 gap-3">
              {[
                { icon: '🔴', title: 'Breakpoints', desc: 'Pause requests and modify them before they reach the server' },
                { icon: '🎭', title: 'Mocks', desc: 'Return custom responses instead of hitting real servers' },
                { icon: '✏️', title: 'Rewrites', desc: 'Automatically modify headers, URLs, or bodies' },
                { icon: '🐢', title: 'Throttling', desc: 'Simulate slow network conditions' },
                { icon: '📜', title: 'Scripts', desc: 'Automate with JavaScript hooks' },
                { icon: '🔌', title: 'Plugins', desc: 'Extend functionality with custom plugins' },
              ].map(f => (
                <div key={f.title} className="flex items-start gap-3 p-3 bg-muted rounded-lg">
                  <span className="text-xl">{f.icon}</span>
                  <div>
                    <h4 className="font-medium">{f.title}</h4>
                    <p className="text-sm text-muted-foreground">{f.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )

      case 'tips':
        return (
          <div className="space-y-4">
            <p className="text-muted-foreground">
              Tips and tricks for power users.
            </p>
            <div className="space-y-3">
              {[
                { title: 'Keyboard Shortcuts', tip: 'Use Ctrl+K to quickly search traffic, Ctrl+B to toggle breakpoints panel' },
                { title: 'Export Sessions', tip: 'Export your traffic as HAR files to share with team members' },
                { title: 'Filter Syntax', tip: 'Use regex patterns like /api/.* to filter requests by URL' },
                { title: 'Replay Requests', tip: 'Right-click any request to replay it or save for later' },
                { title: 'Dark Mode', tip: 'Toggle dark mode in the header for comfortable debugging at night' },
              ].map(t => (
                <div key={t.title} className="p-3 bg-muted rounded-lg">
                  <h4 className="font-medium flex items-center gap-2">
                    <Lightbulb className="h-4 w-4 text-yellow-500" />
                    {t.title}
                  </h4>
                  <p className="text-sm text-muted-foreground mt-1">{t.tip}</p>
                </div>
              ))}
            </div>
          </div>
        )

      default:
        return <p>Unknown step</p>
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <Card className="w-full max-w-2xl max-h-[90vh] overflow-auto">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>{step.title}</CardTitle>
              <CardDescription>
                Step {currentStep + 1} of {status.total_steps}
              </CardDescription>
            </div>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex gap-1 mt-4">
            {status.steps.map((s, i) => (
              <button
                key={s.id}
                onClick={() => setCurrentStep(i)}
                className={`flex-1 h-2 rounded-full transition-colors ${
                  i === currentStep ? 'bg-primary' :
                  s.completed ? 'bg-green-500' : 'bg-muted'
                }`}
              />
            ))}
          </div>
        </CardHeader>

        <CardContent>
          {renderStepContent(step.id)}
        </CardContent>

        <CardFooter className="flex justify-between">
          <div>
            {!step.optional && (
              <Button variant="ghost" onClick={skipOnboarding}>
                Skip Setup
              </Button>
            )}
          </div>
          <div className="flex gap-2">
            {!isFirstStep && (
              <Button variant="outline" onClick={() => setCurrentStep(currentStep - 1)}>
                <ChevronLeft className="h-4 w-4 mr-1" />
                Back
              </Button>
            )}
            {isLastStep ? (
              <Button onClick={() => {
                completeStep(step.id)
                onClose()
              }}>
                Get Started
                <CheckCircle className="h-4 w-4 ml-1" />
              </Button>
            ) : (
              <Button onClick={() => {
                completeStep(step.id)
                setCurrentStep(currentStep + 1)
              }}>
                Next
                <ChevronRight className="h-4 w-4 ml-1" />
              </Button>
            )}
          </div>
        </CardFooter>
      </Card>
    </div>
  )
}
