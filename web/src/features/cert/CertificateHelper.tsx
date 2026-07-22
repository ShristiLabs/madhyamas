import { useState, useEffect } from "react";
import { QRCodeSVG } from "qrcode.react";
import {
  Download,
  Shield,
  AlertCircle,
  CheckCircle,
  Copy,
  Wifi,
  Monitor,
  Smartphone,
  Apple,
  Terminal,
  Globe,
  Info,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/use-toast";
import { apiGet, apiGetRaw } from "@/lib/api/client";

interface CertificateHelperProps {
  trigger?: React.ReactNode;
}

interface ProxyConfig {
  ip: string;
  port: number;
  certUrl: string;
}

// Platform instruction type
interface InstructionStep {
  title: string;
  description: string;
  subSteps?: string[];
  code?: string;
}

export function CertificateHelper({ trigger }: CertificateHelperProps) {
  const [downloading, setDownloading] = useState(false);
  const [proxyConfig, setProxyConfig] = useState<ProxyConfig>({
    ip: "Detecting...",
    port: 8888,
    certUrl: "",
  });
  const { toast } = useToast();

  // Check if a value is a usable IP address (not localhost/127.x.x.x)
  const isUsableIP = (ip: string): boolean => {
    if (!ip || ip === "localhost" || ip === "0.0.0.0") return false;
    if (ip.startsWith("127.")) return false;
    // Check if it's a valid IP address format
    const ipRegex = /^(\d{1,3}\.){3}\d{1,3}$/;
    return ipRegex.test(ip);
  };

  // Check if IP is a private/local network IP
  const isPrivateIP = (ip: string): boolean => {
    if (!ip) return false;
    const parts = ip.split(".").map(Number);
    if (parts.length !== 4) return false;

    // 10.0.0.0 - 10.255.255.255
    if (parts[0] === 10) return true;

    // 172.16.0.0 - 172.31.255.255
    if (parts[0] === 172 && parts[1] >= 16 && parts[1] <= 31) return true;

    // 192.168.0.0 - 192.168.255.255
    if (parts[0] === 192 && parts[1] === 168) return true;

    return false;
  };

  // Detect local IP address
  useEffect(() => {
    const detectIP = async () => {
      let proxyPort = 8888;
      let apiPort = 3001;

      // Get config from backend API (backend now detects private IP)
      try {
        const config = await apiGet<{
          proxy_port?: number;
          api_port?: number;
          host?: string;
        }>("/config");
        proxyPort = config.proxy_port || 8888;
        apiPort = config.api_port || 3001;
        // Backend now returns detected private IP, so we can use it directly
        if (config.host && isUsableIP(config.host)) {
          setProxyConfig({
            ip: config.host,
            port: proxyPort,
            certUrl: `http://${config.host}:${apiPort}/api/cert/ca`,
          });
          return;
        }
      } catch {
        // API config not available
      }

      // Try WebRTC to detect local IP
      try {
        // Use empty iceServers array to only get local network IPs
        const pc = new RTCPeerConnection({
          iceServers: [],
        });

        pc.createDataChannel("");

        const detectedIPs: string[] = [];
        const candidatePromise = new Promise<string | null>((resolve) => {
          pc.onicecandidate = (ice) => {
            if (!ice || !ice.candidate) {
              // When gathering is complete, pick the best IP
              if (detectedIPs.length > 0) {
                // Prioritize private IPs over public IPs
                const privateIP = detectedIPs.find((ip) => isPrivateIP(ip));
                const selectedIP = privateIP || detectedIPs[0];
                resolve(selectedIP);
              } else {
                resolve(null);
              }
              return;
            }

            const candidate = ice.candidate.candidate;
            const ipMatch = candidate.match(/(\d{1,3}\.){3}\d{1,3}/);
            if (ipMatch) {
              const ip = ipMatch[0];
              if (isUsableIP(ip) && !detectedIPs.includes(ip)) {
                detectedIPs.push(ip);
              }
            }
          };

          // Timeout after 2 seconds (faster since we're only checking local)
          setTimeout(() => {
            pc.close();
            if (detectedIPs.length > 0) {
              // Prioritize private IPs over public IPs
              const privateIP = detectedIPs.find((ip) => isPrivateIP(ip));
              const selectedIP = privateIP || detectedIPs[0];
              resolve(selectedIP);
            } else {
              resolve(null);
            }
          }, 2000);
        });

        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);

        const detectedIP = await candidatePromise;

        if (detectedIP) {
          setProxyConfig({
            ip: detectedIP,
            port: proxyPort,
            certUrl: `http://${detectedIP}:${apiPort}/api/cert/ca`,
          });
        } else {
          // Fallback: use hostname if it's a usable IP
          const hostname = window.location.hostname;
          if (isUsableIP(hostname)) {
            setProxyConfig({
              ip: hostname,
              port: proxyPort,
              certUrl: `http://${hostname}:${apiPort}/api/cert/ca`,
            });
          } else {
            // Last resort: show placeholder instructing user to find their IP
            setProxyConfig({
              ip: "Your computer's IP",
              port: proxyPort,
              certUrl: "",
            });
          }
        }
      } catch (error) {
        console.error("Failed to detect IP:", error);
        const hostname = window.location.hostname;
        if (isUsableIP(hostname)) {
          setProxyConfig({
            ip: hostname,
            port: proxyPort,
            certUrl: `http://${hostname}:${apiPort}/api/cert/ca`,
          });
        } else {
          setProxyConfig({
            ip: "Your computer's IP",
            port: proxyPort,
            certUrl: "",
          });
        }
      }
    };

    detectIP();
  }, []);

  const handleDownload = async () => {
    setDownloading(true);
    try {
      const response = await apiGetRaw("/cert/ca");
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "madhyamas-ca.crt";
      a.click();
      URL.revokeObjectURL(url);

      toast({
        title: "Certificate Downloaded",
        description: "Install this certificate to enable HTTPS interception.",
      });
    } catch (error) {
      console.error("Failed to download certificate:", error);
      toast({
        title: "Download Failed",
        description: "Could not download certificate.",
        variant: "destructive",
      });
    } finally {
      setDownloading(false);
    }
  };

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    toast({ description: `${label} copied to clipboard` });
  };

  // Platform-specific proxy setup and certificate installation instructions
  const platformInstructions: Record<
    string,
    { icon: React.ReactNode; instructions: InstructionStep[] }
  > = {
    macOS: {
      icon: <Apple className="h-4 w-4" />,
      instructions: [
        {
          title: "1. Configure HTTP Proxy",
          description: "Set up system-wide proxy settings",
          subSteps: [
            "Open System Settings (or System Preferences on older macOS)",
            'Click on "Network"',
            "Select your active network connection (Wi-Fi or Ethernet)",
            'Click "Details..." button',
            'Navigate to the "Proxies" tab',
            'Check both "Web Proxy (HTTP)" and "Secure Web Proxy (HTTPS)"',
            `Enter Server: ${proxyConfig.ip}`,
            `Enter Port: ${proxyConfig.port}`,
            'Leave "Proxy server requires password" unchecked',
            'Click "OK" to save',
          ],
        },
        {
          title: "2. Download CA Certificate",
          description:
            "Download the Madhyamas CA certificate using the button above or visit the certificate URL in Safari",
        },
        {
          title: "3. Install Certificate",
          description: "Add the certificate to your keychain",
          subSteps: [
            'Double-click the downloaded "madhyamas-ca.crt" file',
            'In the "Add Certificates" dialog, select "System" from the Keychain dropdown',
            'Click "Add" to install the certificate',
            "Enter your macOS password when prompted",
          ],
        },
        {
          title: "4. Trust the Certificate",
          description: "Mark the certificate as trusted for SSL",
          subSteps: [
            'Open "Keychain Access" app (use Spotlight or find in Applications > Utilities)',
            'Select "System" keychain in the left sidebar',
            'Find "Madhyamas CA" certificate in the list',
            "Double-click the certificate to open details",
            'Expand the "Trust" section',
            'Set "When using this certificate" to "Always Trust"',
            "Close the window and enter your password to confirm",
          ],
        },
        {
          title: "5. Verify Setup",
          description: "Test that HTTPS interception is working",
          subSteps: [
            "Open Safari or any browser",
            "Visit any HTTPS website (e.g., https://example.com)",
            "You should NOT see any certificate warnings",
            "Check Madhyamas UI to see captured traffic",
          ],
        },
      ],
    },
    windows: {
      icon: <Monitor className="h-4 w-4" />,
      instructions: [
        {
          title: "1. Configure HTTP Proxy",
          description: "Set up system-wide proxy settings",
          subSteps: [
            "Press Win + I to open Windows Settings",
            'Go to "Network & Internet"',
            'Click on "Proxy" in the left sidebar',
            'Scroll to "Manual proxy setup" section',
            'Toggle "Use a proxy server" to ON',
            `Enter Address: ${proxyConfig.ip}`,
            `Enter Port: ${proxyConfig.port}`,
            '(Optional) Add "localhost;127.0.0.1" to "Don\'t use proxy for" field',
            'Click "Save" button',
          ],
        },
        {
          title: "2. Download CA Certificate",
          description:
            "Download the Madhyamas CA certificate using the button above",
        },
        {
          title: "3. Install Certificate",
          description: "Import the certificate into Windows Certificate Store",
          subSteps: [
            'Double-click the downloaded "madhyamas-ca.crt" file',
            'Click "Install Certificate..." button',
            'Select "Local Machine" (requires administrator privileges)',
            'Click "Next"',
            'Select "Place all certificates in the following store"',
            'Click "Browse..." and select "Trusted Root Certification Authorities"',
            'Click "Next", then "Finish"',
            'Click "Yes" on the security warning dialog',
            'You should see "The import was successful" message',
          ],
        },
        {
          title: "4. Verify Installation",
          description: "Confirm the certificate is properly installed",
          subSteps: [
            'Press Win + R, type "certmgr.msc" and press Enter',
            'Expand "Trusted Root Certification Authorities"',
            'Click on "Certificates" folder',
            'Look for "Madhyamas CA" in the list',
            "Double-click to view certificate details",
          ],
        },
        {
          title: "5. Test HTTPS Interception",
          description: "Verify that the setup is working",
          subSteps: [
            "Open any browser (Chrome, Edge, Firefox)",
            "Visit an HTTPS website (e.g., https://example.com)",
            "You should NOT see certificate warnings",
            "Check Madhyamas UI for captured HTTPS traffic",
          ],
        },
      ],
    },
    linux: {
      icon: <Terminal className="h-4 w-4" />,
      instructions: [
        {
          title: "1. Configure HTTP Proxy (GUI Method)",
          description: "Set up proxy using system settings",
          subSteps: [
            "Open Settings application",
            'Go to "Network" or "Network & Internet"',
            'Click on "Network Proxy" or "Proxy"',
            'Select "Manual" configuration',
            `HTTP Proxy: ${proxyConfig.ip}`,
            `Port: ${proxyConfig.port}`,
            `HTTPS Proxy: ${proxyConfig.ip}`,
            `Port: ${proxyConfig.port}`,
            'Click "Apply" or "Apply system wide"',
          ],
        },
        {
          title: "1. Configure HTTP Proxy (Terminal Method)",
          description: "Alternative: Set proxy using environment variables",
          code: `# Add to ~/.bashrc or ~/.zshrc for persistence
export http_proxy="http://${proxyConfig.ip}:${proxyConfig.port}"
export https_proxy="http://${proxyConfig.ip}:${proxyConfig.port}"
export HTTP_PROXY="http://${proxyConfig.ip}:${proxyConfig.port}"
export HTTPS_PROXY="http://${proxyConfig.ip}:${proxyConfig.port}"

# Apply immediately
source ~/.bashrc  # or source ~/.zshrc`,
        },
        {
          title: "2. Download CA Certificate",
          description: "Download the certificate file",
          code: `# Using wget
wget http://${proxyConfig.ip}:3001/api/cert/ca -O madhyamas-ca.crt

# Or using curl
curl http://${proxyConfig.ip}:3001/api/cert/ca -o madhyamas-ca.crt`,
        },
        {
          title: "3. Install Certificate (Ubuntu/Debian)",
          description: "Add certificate to system trust store",
          code: `# Copy certificate to system CA directory
sudo cp madhyamas-ca.crt /usr/local/share/ca-certificates/madhyamas-ca.crt

# Update CA certificates
sudo update-ca-certificates

# Verify installation
ls -la /etc/ssl/certs/ | grep madhyamas`,
        },
        {
          title: "3. Install Certificate (Fedora/RHEL/CentOS)",
          description: "Add certificate to system trust store",
          code: `# Copy certificate to system CA directory
sudo cp madhyamas-ca.crt /etc/pki/ca-trust/source/anchors/madhyamas-ca.crt

# Update CA trust
sudo update-ca-trust

# Verify installation
trust list | grep -i madhyamas`,
        },
        {
          title: "4. Install Certificate for Browsers",
          description: "Some browsers use their own certificate store",
          subSteps: [
            "For Firefox: Settings > Privacy & Security > Certificates > View Certificates > Authorities > Import",
            "For Chrome: Settings > Privacy and security > Security > Manage certificates > Authorities > Import",
            "Select the madhyamas-ca.crt file",
            'Check "Trust this CA to identify websites"',
            "Click OK",
          ],
        },
        {
          title: "5. Test Configuration",
          description: "Verify the setup is working",
          code: `# Test with curl
curl -v https://example.com

# Should show Madhyamas CA in certificate chain
# No SSL certificate errors should appear`,
        },
      ],
    },
    ios: {
      icon: <Smartphone className="h-4 w-4" />,
      instructions: [
        {
          title: "1. Configure HTTP Proxy",
          description: "Set up Wi-Fi proxy settings",
          subSteps: [
            "Open the Settings app",
            'Tap on "Wi-Fi"',
            "Tap the (i) info icon next to your connected Wi-Fi network",
            'Scroll down to "HTTP Proxy" section',
            'Tap "Configure Proxy"',
            'Select "Manual"',
            `Enter Server: ${proxyConfig.ip}`,
            `Enter Port: ${proxyConfig.port}`,
            "Leave Authentication OFF",
            'Tap "Save" in the top right',
          ],
        },
        {
          title: "2. Download CA Certificate",
          description: "Install the certificate profile on your device",
          subSteps: [
            "Scan the QR code above using your Camera app",
            "Or open Safari and visit the certificate URL",
            'Tap "Allow" when prompted to download configuration profile',
            'A "Profile Downloaded" notification will appear',
          ],
        },
        {
          title: "3. Install Certificate Profile",
          description: "Add the certificate to your device",
          subSteps: [
            "Open the Settings app",
            'You should see "Profile Downloaded" at the top',
            'Tap on "Profile Downloaded"',
            'Tap "Install" in the top right',
            "Enter your device passcode if prompted",
            'Tap "Install" again to confirm',
            'Tap "Install" once more in the warning dialog',
            'Tap "Done" when installation completes',
          ],
        },
        {
          title: "4. Enable Full Trust for Certificate",
          description: "This critical step enables HTTPS interception",
          subSteps: [
            "Open Settings app",
            'Go to "General"',
            'Scroll down and tap "About"',
            'Scroll to bottom and tap "Certificate Trust Settings"',
            'Under "Enable Full Trust for Root Certificates"',
            'Toggle ON the switch for "Madhyamas CA"',
            'Tap "Continue" in the warning dialog',
          ],
        },
        {
          title: "5. Verify Setup",
          description: "Test that HTTPS interception is working",
          subSteps: [
            "Open Safari browser",
            "Visit any HTTPS website (e.g., https://example.com)",
            "You should NOT see any certificate warnings",
            "Check Madhyamas UI on your computer to see captured traffic",
            "If you see warnings, repeat step 4 to enable certificate trust",
          ],
        },
      ],
    },
    android: {
      icon: <Smartphone className="h-4 w-4" />,
      instructions: [
        {
          title: "1. Configure HTTP Proxy",
          description: "Set up Wi-Fi proxy settings",
          subSteps: [
            "Open Settings app",
            'Tap on "Wi-Fi" or "Network & Internet"',
            "Long-press on your connected Wi-Fi network",
            'Tap "Modify network" or "Manage network settings"',
            'Tap "Advanced options" to expand',
            'Set Proxy to "Manual"',
            `Enter Proxy hostname: ${proxyConfig.ip}`,
            `Enter Proxy port: ${proxyConfig.port}`,
            'Tap "Save"',
          ],
        },
        {
          title: "2. Download CA Certificate",
          description: "Get the certificate file on your device",
          subSteps: [
            "Scan the QR code above using your Camera app",
            "Or open Chrome and visit the certificate URL",
            "The certificate file will download automatically",
            "Note the download location (usually Downloads folder)",
          ],
        },
        {
          title: "3. Install CA Certificate",
          description: "Add certificate to trusted credentials",
          subSteps: [
            "Open Settings app",
            'Go to "Security" or "Security & Privacy"',
            'Tap "Encryption & credentials" or "More security settings"',
            'Tap "Install a certificate" or "Install from storage"',
            'Select "CA certificate"',
            'Tap "Install anyway" if warned',
            "Navigate to Downloads folder",
            'Select "madhyamas-ca.crt" file',
            'Enter a name like "Madhyamas CA" if prompted',
            'Tap "OK"',
          ],
        },
        {
          title: "4. Verify Certificate Installation",
          description: "Confirm the certificate is installed",
          subSteps: [
            "Go to Settings > Security > Trusted credentials",
            'Tap on "User" tab',
            'Look for "Madhyamas CA" in the list',
            "Tap on it to view details",
          ],
        },
        {
          title: "5. Configure Apps for User Certificates (Android 7+)",
          description: "Some apps require additional configuration",
          subSteps: [
            "Note: Android 7+ apps may not trust user certificates by default",
            "For Chrome: Should work automatically",
            "For other apps: May need to be configured individually",
            "System apps and browsers should work with the installed certificate",
          ],
        },
        {
          title: "6. Test HTTPS Interception",
          description: "Verify the setup is working",
          subSteps: [
            "Open Chrome browser",
            "Visit an HTTPS website (e.g., https://example.com)",
            "You should NOT see certificate warnings",
            "Check Madhyamas UI on your computer for captured traffic",
          ],
        },
      ],
    },
    firefox: {
      icon: <Globe className="h-4 w-4" />,
      instructions: [
        {
          title: "1. Configure HTTP Proxy",
          description: "Set up Firefox proxy settings",
          subSteps: [
            "Click the menu button (☰) in the top right",
            'Click "Settings" or "Preferences"',
            'Scroll down to "Network Settings" section',
            'Click "Settings..." button',
            'Select "Manual proxy configuration"',
            `HTTP Proxy: ${proxyConfig.ip}`,
            `Port: ${proxyConfig.port}`,
            'Check "Also use this proxy for HTTPS"',
            `SSL Proxy: ${proxyConfig.ip}`,
            `Port: ${proxyConfig.port}`,
            'Leave "No Proxy for" as default or add "localhost, 127.0.0.1"',
            'Click "OK" to save',
          ],
        },
        {
          title: "2. Download CA Certificate",
          description:
            "Download the Madhyamas CA certificate using the button above",
        },
        {
          title: "3. Install Certificate in Firefox",
          description: "Import the certificate into Firefox certificate store",
          subSteps: [
            'In Firefox Settings, scroll to "Privacy & Security"',
            'Scroll down to "Certificates" section',
            'Click "View Certificates..." button',
            'Go to "Authorities" tab',
            'Click "Import..." button',
            'Select the downloaded "madhyamas-ca.crt" file',
            'Check "Trust this CA to identify websites"',
            'Check "Trust this CA to identify email users" (optional)',
            'Click "OK"',
          ],
        },
        {
          title: "4. Verify Certificate Installation",
          description: "Confirm the certificate is properly installed",
          subSteps: [
            'In the Certificates window, stay on "Authorities" tab',
            'Look for "Madhyamas CA" in the list',
            "It should be under organization name",
            "Double-click to view certificate details",
          ],
        },
        {
          title: "5. Test HTTPS Interception",
          description: "Verify that the setup is working",
          subSteps: [
            "Visit an HTTPS website (e.g., https://example.com)",
            "You should NOT see any certificate warnings",
            "Click the padlock icon in the address bar",
            'Click "Connection secure" > "More information"',
            "Verify the certificate chain includes Madhyamas CA",
            "Check Madhyamas UI for captured HTTPS traffic",
          ],
        },
      ],
    },
    chrome: {
      icon: <Globe className="h-4 w-4" />,
      instructions: [
        {
          title: "1. Configure HTTP Proxy",
          description: "Chrome uses system proxy settings",
          subSteps: [
            "Windows: Follow the Windows proxy setup instructions above",
            "macOS: Follow the macOS proxy setup instructions above",
            "Linux: Follow the Linux proxy setup instructions above",
            "Or use Chrome with command-line flags:",
          ],
          code: `# Launch Chrome with proxy
chrome --proxy-server="${proxyConfig.ip}:${proxyConfig.port}"

# Or on macOS
open -a "Google Chrome" --args --proxy-server="${proxyConfig.ip}:${proxyConfig.port}"`,
        },
        {
          title: "2. Download CA Certificate",
          description:
            "Download the Madhyamas CA certificate using the button above",
        },
        {
          title: "3. Install Certificate in Chrome",
          description: "Import the certificate into Chrome certificate store",
          subSteps: [
            "Click the three-dot menu (⋮) in the top right",
            'Go to "Settings"',
            'Click "Privacy and security" in the left sidebar',
            'Click "Security"',
            'Scroll down and click "Manage certificates"',
            'Windows: Go to "Trusted Root Certification Authorities" tab',
            "macOS: This opens Keychain Access (follow macOS instructions)",
            'Linux: Go to "Authorities" tab',
            'Click "Import" button',
            'Select the downloaded "madhyamas-ca.crt" file',
            'Check "Trust this certificate for identifying websites"',
            'Click "OK" or "Import"',
          ],
        },
        {
          title: "4. Restart Chrome",
          description: "Close and reopen Chrome for changes to take effect",
          subSteps: [
            "Close all Chrome windows completely",
            "Reopen Chrome",
            "The certificate should now be trusted",
          ],
        },
        {
          title: "5. Test HTTPS Interception",
          description: "Verify that the setup is working",
          subSteps: [
            "Visit an HTTPS website (e.g., https://example.com)",
            'You should NOT see "Your connection is not private" warnings',
            "Click the padlock icon in the address bar",
            'Click "Connection is secure"',
            'Click "Certificate is valid"',
            "Verify the certificate chain includes Madhyamas CA",
            "Check Madhyamas UI for captured HTTPS traffic",
          ],
        },
      ],
    },
  };

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
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            HTTPS Proxy Setup
          </DialogTitle>
          <DialogDescription>
            Configure your devices to use Madhyamas for HTTP/HTTPS traffic
            interception
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto py-4 space-y-6">
          {/* Proxy Configuration Card with QR Code */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Proxy Info */}
            <div className="space-y-4">
              <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                <Wifi className="h-4 w-4" />
                Proxy Configuration
              </div>

              <div className="p-4 bg-muted/50 rounded-lg border space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    Proxy IP
                  </span>
                  <div className="flex items-center gap-2">
                    <code className="px-2 py-1 bg-background rounded font-mono text-sm">
                      {proxyConfig.ip}
                    </code>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 p-0"
                      onClick={() =>
                        copyToClipboard(proxyConfig.ip, "Proxy IP")
                      }
                    >
                      <Copy className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    Proxy Port
                  </span>
                  <div className="flex items-center gap-2">
                    <code className="px-2 py-1 bg-background rounded font-mono text-sm">
                      {proxyConfig.port}
                    </code>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 p-0"
                      onClick={() =>
                        copyToClipboard(String(proxyConfig.port), "Proxy Port")
                      }
                    >
                      <Copy className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    Certificate URL
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 w-7 p-0"
                    onClick={() =>
                      copyToClipboard(proxyConfig.certUrl, "Certificate URL")
                    }
                  >
                    <Copy className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>

              {/* Download Button */}
              <Button
                onClick={handleDownload}
                disabled={downloading}
                className="w-full"
              >
                <Download className="h-4 w-4 mr-2" />
                {downloading ? "Downloading..." : "Download CA Certificate"}
              </Button>
            </div>

            {/* QR Code */}
            <div className="flex flex-col items-center justify-center p-4 bg-white rounded-lg border">
              <div className="text-sm font-medium text-muted-foreground mb-3 text-center">
                <Smartphone className="h-4 w-4 inline mr-1" />
                Scan to Download Certificate
              </div>
              {proxyConfig.certUrl ? (
                <QRCodeSVG
                  value={proxyConfig.certUrl}
                  size={160}
                  level="M"
                  bgColor="#ffffff"
                  fgColor="#000000"
                />
              ) : (
                <div className="w-40 h-40 bg-muted animate-pulse rounded" />
              )}
              <p className="text-xs text-muted-foreground mt-3 text-center">
                Scan with your mobile device camera
              </p>
            </div>
          </div>

          {/* Platform Setup Instructions */}
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Info className="h-4 w-4" />
              Setup Instructions by Platform
            </div>

            <Tabs defaultValue="ios" className="w-full">
              <TabsList className="w-full justify-start flex-wrap h-auto gap-1">
                {Object.entries(platformInstructions).map(
                  ([platform, { icon }]) => (
                    <TabsTrigger
                      key={platform}
                      value={platform}
                      className="capitalize"
                    >
                      {icon}
                      <span className="ml-1.5">{platform}</span>
                    </TabsTrigger>
                  ),
                )}
              </TabsList>

              {Object.entries(platformInstructions).map(
                ([platform, { instructions }]) => (
                  <TabsContent key={platform} value={platform} className="mt-4">
                    <div className="space-y-4">
                      {instructions.map((step, index) => (
                        <div key={index} className="flex gap-3">
                          <div className="flex-shrink-0 w-6 h-6 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-xs font-medium">
                            {index + 1}
                          </div>
                          <div className="flex-1 min-w-0">
                            <h4 className="font-medium text-sm">
                              {step.title}
                            </h4>
                            <p className="text-sm text-muted-foreground mt-0.5">
                              {step.description}
                            </p>
                            {step.subSteps && (
                              <ul className="mt-2 space-y-1">
                                {step.subSteps.map((subStep, subIndex) => (
                                  <li
                                    key={subIndex}
                                    className="flex items-start gap-2 text-sm"
                                  >
                                    <CheckCircle className="h-4 w-4 text-green-500 flex-shrink-0 mt-0.5" />
                                    <span>{subStep}</span>
                                  </li>
                                ))}
                              </ul>
                            )}
                            {step.code && (
                              <pre className="mt-2 p-3 bg-muted rounded-md text-xs font-mono overflow-x-auto">
                                {step.code}
                              </pre>
                            )}
                          </div>
                        </div>
                      ))}
                    </div>
                  </TabsContent>
                ),
              )}
            </Tabs>
          </div>

          {/* Warning */}
          <div className="bg-yellow-50 dark:bg-yellow-950 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4">
            <div className="flex gap-2">
              <AlertCircle className="h-5 w-5 text-yellow-600 dark:text-yellow-500 flex-shrink-0" />
              <div className="text-sm">
                <p className="font-medium text-yellow-800 dark:text-yellow-200">
                  Security Notice
                </p>
                <p className="text-yellow-700 dark:text-yellow-300 mt-1">
                  Only install this certificate on devices you control. Remove
                  it when done debugging. This certificate allows Madhyamas to
                  decrypt HTTPS traffic.
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
                  After setting up, try accessing an HTTPS site. If you see
                  certificate warnings, the installation was not successful.
                </p>
              </div>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
