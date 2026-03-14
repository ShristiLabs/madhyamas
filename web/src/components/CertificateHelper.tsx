import { useState } from 'react'
import { Download, Shield, AlertCircle, CheckCircle } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from './ui/dialog'
import { Button } from './ui/button'

interface CertificateHelperProps {
  trigger?: React.ReactNode
}

export function CertificateHelper({ trigger }: CertificateHelperProps) {
  const [downloading, setDownloading] = useState(false)

  const handleDownload = async () => {
    setDownloading(true)
    try {
      const response = await fetch('/api/cert/ca')
      if (!response.ok) throw new Error('Failed to download certificate')

      const blob = await response.blob()
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'proxyforge-ca.pem'
      a.click()
      URL.revokeObjectURL(url)
    } catch (error) {
      console.error('Failed to download certificate:', error)
    } finally {
      setDownloading(false)
    }
  }

  return (
    <Dialog>
      <DialogTrigger asChild>
        {trigger || (
          <Button variant="ghost" size="sm">
            <Shield className="h-4 w-4 mr-2" />
            HTTPS Setup
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            HTTPS Interception Setup
          </DialogTitle>
          <DialogDescription>
            To intercept and inspect HTTPS traffic, you need to install the ProxyForge CA certificate.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Step 1: Download */}
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <div className="w-6 h-6 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-sm font-semibold">
                1
              </div>
              <h3 className="font-semibold">Download Certificate</h3>
            </div>
            <div className="ml-8">
              <Button onClick={handleDownload} disabled={downloading}>
                <Download className="h-4 w-4 mr-2" />
                {downloading ? 'Downloading...' : 'Download CA Certificate'}
              </Button>
            </div>
          </div>

          {/* Step 2: Install Instructions */}
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <div className="w-6 h-6 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-sm font-semibold">
                2
              </div>
              <h3 className="font-semibold">Install Certificate</h3>
            </div>

            <div className="ml-8 space-y-4">
              {/* macOS */}
              <div className="border rounded-lg p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <span className="text-lg">🍎</span> macOS
                </h4>
                <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
                  <li>Open the downloaded certificate file</li>
                  <li>It will open in Keychain Access</li>
                  <li>Add it to the "System" keychain</li>
                  <li>Find "ProxyForge Root CA" and double-click</li>
                  <li>Set "When using this certificate" to "Always Trust"</li>
                  <li>Restart your browser</li>
                </ol>
              </div>

              {/* Windows */}
              <div className="border rounded-lg p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <span className="text-lg">🪟</span> Windows
                </h4>
                <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
                  <li>Double-click the downloaded certificate</li>
                  <li>Click "Install Certificate"</li>
                  <li>Select "Local Machine" and click Next</li>
                  <li>Select "Place all certificates in the following store"</li>
                  <li>Browse to "Trusted Root Certification Authorities"</li>
                  <li>Click Finish and restart your browser</li>
                </ol>
              </div>

              {/* Linux */}
              <div className="border rounded-lg p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <span className="text-lg">🐧</span> Linux
                </h4>
                <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
                  <li>Copy the certificate to /usr/local/share/ca-certificates/</li>
                  <li>Run: <code className="bg-muted px-1 rounded">sudo update-ca-certificates</code></li>
                  <li>For browsers, import via Settings → Certificates → Authorities</li>
                </ol>
              </div>

              {/* Firefox */}
              <div className="border rounded-lg p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <span className="text-lg">🦊</span> Firefox (All Platforms)
                </h4>
                <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
                  <li>Go to Settings → Privacy & Security</li>
                  <li>Scroll to "Certificates" and click "View Certificates"</li>
                  <li>Go to "Authorities" tab</li>
                  <li>Click "Import" and select the certificate</li>
                  <li>Check "Trust this CA to identify websites"</li>
                  <li>Click OK</li>
                </ol>
              </div>
            </div>
          </div>

          {/* Warning */}
          <div className="bg-yellow-50 dark:bg-yellow-950 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4">
            <div className="flex gap-2">
              <AlertCircle className="h-5 w-5 text-yellow-600 dark:text-yellow-500 flex-shrink-0" />
              <div className="text-sm">
                <p className="font-medium text-yellow-800 dark:text-yellow-200">Security Notice</p>
                <p className="text-yellow-700 dark:text-yellow-300 mt-1">
                  Only install this certificate if you trust this ProxyForge instance.
                  This certificate allows the proxy to decrypt HTTPS traffic.
                </p>
              </div>
            </div>
          </div>

          {/* Success Check */}
          <div className="bg-muted rounded-lg p-4">
            <div className="flex gap-2">
              <CheckCircle className="h-5 w-5 text-green-600 dark:text-green-500 flex-shrink-0" />
              <div className="text-sm">
                <p className="font-medium">Verify Installation</p>
                <p className="text-muted-foreground mt-1">
                  After installing, try accessing an HTTPS site through the proxy.
                  If you see a certificate warning, the installation was not successful.
                </p>
              </div>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
