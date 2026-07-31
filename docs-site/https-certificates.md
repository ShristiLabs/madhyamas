# HTTPS & Certificates

Modern web traffic is encrypted with HTTPS. To inspect HTTPS traffic, Madhyamas acts as a "man-in-the-middle" — it terminates the TLS connection from your client, decrypts the traffic, then establishes a new connection to the server. This requires a Certificate Authority (CA) certificate that your client trusts.

## How HTTPS Interception Works

1. **Client connects to Madhyamas proxy** and requests an HTTPS tunnel
2. **Madhyamas generates a certificate** for the target domain, signed by its local CA
3. **Client verifies the certificate** against its trust store — if the Madhyamas CA is installed, the connection proceeds
4. **Madhyamas connects to the real server** and relays traffic, decrypting and re-encrypting as needed

Without the CA certificate installed, your browser or app will show certificate warnings and refuse to connect.

## Installing the CA Certificate

The easiest way to install the certificate is through the **Setup** dialog — click the **Setup** button (gear icon) in the top toolbar.

![Setup Dialog](/screenshots/setup-dialog.png)

The dialog provides step-by-step instructions and a download button for the CA certificate. You can also install it manually using the platform-specific instructions below.

### macOS

```bash
# Download the certificate
curl -o ~/Downloads/madhyamas-ca.pem http://localhost:3001/api/cert/ca

# Install and trust it
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ~/Downloads/madhyamas-ca.pem
```

Or manually:
1. Download the certificate from `http://localhost:3001/api/cert/ca`
2. Double-click the `.pem` file to open it in Keychain Access
3. Find "Madhyamas Root CA" → right-click → Get Info → Trust → "Always Trust"

### Windows

```powershell
# Download the certificate
Invoke-WebRequest -Uri "http://localhost:3001/api/cert/ca" -OutFile "$env:USERPROFILE\Downloads\madhyamas-ca.pem"

# Install to trusted root store
Import-Certificate -FilePath "$env:USERPROFILE\Downloads\madhyamas-ca.pem" `
  -CertStoreLocation Cert:\LocalMachine\Root
```

### Linux (Ubuntu/Debian)

```bash
curl -o /tmp/madhyamas-ca.pem http://localhost:3001/api/cert/ca
sudo cp /tmp/madhyamas-ca.pem /usr/local/share/ca-certificates/madhyamas-ca.crt
sudo update-ca-certificates
```

### iPhone / iPad

1. Open Safari on your device and visit `http://<your-computer-ip>:3001/api/cert/ca`
2. When prompted, tap **Allow** to download the profile
3. Go to **Settings → Profile Downloaded → Install**
4. Go to **Settings → General → About → Certificate Trust Settings** → enable trust for Madhyamas Root CA

### Android

1. Download the certificate from `http://<your-computer-ip>:3001/api/cert/ca`
2. Go to **Settings → Security → Install a certificate → CA certificate**
3. Select the downloaded file

::: warning Android 7.0+
Apps don't trust user-installed CA certificates by default. To intercept HTTPS traffic from specific apps, you may need root access to install the certificate in the system store, or use the Madhyamas Android companion app.
:::

## Disabling HTTPS Interception

If you only need to inspect HTTP traffic, or if HTTPS interception is causing issues, you can disable it:

```bash
# Via CLI
madhyamas config update --intercept-https false

# Or start with the flag
madhyamas serve --no-https
```

When disabled, HTTPS traffic passes through the proxy as an opaque tunnel — Madhyamas can see the connection but not the content.

## Certificate Pinning

Some mobile apps use **certificate pinning** — they hardcode the expected server certificate and reject any proxy's CA certificate. This is a security feature that prevents man-in-the-middle interception.

If an app uses certificate pinning, you'll see 502 errors in the traffic list with TLS handshake failure messages. Unfortunately, there's no universal bypass — it depends on the app and platform:

- **Android**: See the Android certificate pinning bypass guide (Frida, APK patching, Magisk modules)
- **iOS**: Jailbroken devices can use SSL Kill Switch; non-jailbroken devices may need app-specific approaches
- **Browsers**: Most browsers don't use pinning and will work with the CA certificate installed

## Regenerating the CA Certificate

If your certificate is compromised or you want to start fresh, delete the existing CA and restart Madhyamas:

```bash
# Stop Madhyamas
# Delete the CA certificate and key
rm ~/.madhyamas/certs/madhyamas-ca.pem
rm ~/.madhyamas/certs/madhyamas-ca-key.pem

# Restart — a new CA is auto-generated
madhyamas serve
```

You'll need to install the new CA certificate on all your devices again.
