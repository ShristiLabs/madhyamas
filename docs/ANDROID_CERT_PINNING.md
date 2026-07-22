# Android Certificate Pinning Bypass Guide

This document describes approaches for intercepting HTTPS traffic from
Android apps that implement certificate pinning, in combination with the
Madhyamas proxy and VPN companion app.

## Table of Contents

- [Understanding Certificate Pinning](#understanding-certificate-pinning)
- [Approach 1: Network Security Config (No root)](#approach-1-network-security-config-no-root)
- [Approach 2: APK Patching (No root)](#approach-2-apk-patching-no-root)
- [Approach 3: Frida Runtime Hooking (Root or gadget)](#approach-3-frida-runtime-hooking-root-or-gadget)
- [Approach 4: LSPosed/Xposed Modules (Root)](#approach-4-lsposedxposed-modules-root)
- [Approach 5: Magisk CA Installation (Root)](#approach-5-magisk-ca-installation-root)
- [Approach 6: Flutter Apps](#approach-6-flutter-apps)
- [Approach 7: React Native Apps](#approach-7-react-native-apps)
- [Decision Matrix](#decision-matrix)

## Understanding Certificate Pinning

Certificate pinning is a security mechanism where an app hardcodes (or
"pins") the expected server certificate or public key, refusing to
connect if the server presents a different certificate — even if it's
signed by a trusted CA.

This means that even if you install the Madhyamas CA certificate on the
device, pinned apps will reject the proxy's certificate.

### Types of Pinning

| Type | Implementation | Bypass Difficulty |
|------|---------------|-------------------|
| Network Security Config | XML in APK | Easy — patch XML |
| OkHttp CertificatePinner | Java/Kotlin code | Medium — smali patch or Frida |
| Custom TrustManager | Java/Kotlin code | Medium — smali patch or Frida |
| Native (BoringSSL/OpenSSL) | C/C++ in .so files | Hard — native hooking |
| Flutter | BoringSSL in libflutter.so | Hard — binary patching |
| Cronet | Native Google library | Hard — native hooking |

## Approach 1: Network Security Config (No root)

**Works for**: Apps that only use NSC for pinning (no code-based pinning)

Android 7+ allows apps to specify a Network Security Config XML that
controls which CAs are trusted. If the app uses NSC (not code-based
pinning), you can modify it to trust user certificates.

### Using the VPN companion app

The Madhyamas VPN app already includes a permissive NSC that trusts user
certificates. If the target app also relies on NSC (and doesn't override
it with code-based pinning), installing the Madhyamas CA as a user
certificate may be sufficient.

### Manual APK patching

```bash
# Decompile
apktool d app.apk -o app/

# Create/replace res/xml/network_security_config.xml
cat > app/res/xml/network_security_config.xml << 'EOF'
<?xml version="1.0" encoding="utf-8"?>
<network-security-config>
    <base-config cleartextTrafficPermitted="false">
        <trust-anchors>
            <certificates src="system" />
            <certificates src="user" />
        </trust-anchors>
    </base-config>
</network-security-config>
EOF

# Add reference in AndroidManifest.xml (if not present)
# android:networkSecurityConfig="@xml/network_security_config"

# Recompile and sign
apktool b app -o app-patched.apk
zipalign -v 4 app-patched.apk app-aligned.apk
apksigner sign --ks ~/.android/debug.keystore --ks-pass android app-aligned.apk
```

## Approach 2: APK Patching (No root)

**Works for**: Java/Kotlin-based pinning (OkHttp, custom TrustManager)

### Using apk-mitm (automated)

```bash
npm install -g apk-mitm
apk-mitm target-app.apk
# Output: target-app-mitm.apk
adb install target-app-mitm.apk
```

`apk-mitm` automatically:
- Replaces Network Security Config to trust user CAs
- Patches OkHttp CertificatePinner.check() to no-op
- Patches custom TrustManager.checkServerTrusted() to no-op
- Sets android:debuggable="true"
- Signs the APK

### Limitations

- Does NOT work for Flutter apps (native pinning in libflutter.so)
- Does NOT work for React Native apps using native bridges
- Does NOT work for apps using Cronet
- Apktool may fail on heavily obfuscated or protected APKs

## Approach 3: Frida Runtime Hooking (Root or gadget)

**Works for**: Most Java/Kotlin pinning, some native pinning

Frida is a dynamic instrumentation toolkit that hooks into running
processes. It can bypass pinning at runtime without modifying the APK.

### Prerequisites (rooted device)

```bash
# Download frida-server matching your device architecture
# from https://github.com/frida/frida/releases

# Push and run frida-server
adb push frida-server-<version>-android-arm64 /data/local/tmp/frida-server
adb shell chmod +x /data/local/tmp/frida-server
adb shell su -c /data/local/tmp/frida-server &
```

### Using objection (high-level wrapper)

```bash
pip install objection
objection -g com.target.app explore
# In the objection prompt:
android sslpinning disable
```

### Using HTTP Toolkit's unpinning script (recommended)

```bash
# Download the script
curl -O https://raw.githubusercontent.com/httptoolkit/frida-interception-and-unpinning/main/android/android-certificate-unpinning.js

# Run it
frida -U -f com.target.app -l android-certificate-unpinning.js
```

This script hooks:
- SSLContext.init()
- OkHttp CertificatePinner (all versions)
- Conscrypt TrustManagerImpl
- HttpsURLConnection
- WebViewClient
- TrustManagerFactory
- Custom TrustManager implementations (auto-detect)

### Non-rooted: Frida Gadget injection

For non-rooted devices, inject Frida Gadget into the APK:

```bash
# Using apk-mitm with Frida gadget
apk-mitm --frida-gadget target-app.apk

# Or manually:
# 1. Download frida-gadget for target ABI
# 2. Inject into a .so file using LIEF
# 3. Add a Frida script as libfrida-gadget.script.so
# 4. Repack and sign APK
```

## Approach 4: LSPosed/Xposed Modules (Root)

**Works for**: Java/Kotlin pinning, some native pinning

LSPosed is a framework for system-wide hooking on rooted devices (requires
Magisk + Zygisk).

### Recommended modules

1. **TrustMe** (kirklin/TrustMe) — Most comprehensive
   - 37+ hook targets
   - OkHttp 2.x/3.x/4.x+
   - Conscrypt, TrustManagerFactory
   - WebView, Apache HTTP
   - Network Security Config
   - Cronet
   - Jetpack Compose settings UI

2. **ssl-kill-switch-lsposed** (0xdad0/ssl-kill-switch-lsposed)
   - Java SSL bypass
   - Flutter native bypass (pattern-scan libflutter.so)
   - React Native BoringSSL hooking

3. **SSLUnpinner** (pccr10001/SSLUnpinner)
   - Standard SSL bypass
   - Flutter patching
   - Multi-architecture scanning

### Installation

1. Root device with Magisk
2. Install LSPosed (Zygisk version)
3. Install the module APK
4. Enable the module in LSPosed Manager
5. Select target apps in the module scope
6. Reboot

## Approach 5: Magisk CA Installation (Root)

**Works for**: Apps that use the system trust store (no custom pinning)

Install the Madhyamas CA into the system certificate store so all apps
trust it by default.

### Using MagiskTrustUserCerts

```bash
# 1. Install Madhyamas CA as a user certificate:
#    Settings → Security → Install a certificate → CA certificate

# 2. Install the Magisk module:
#    https://github.com/NVISOsecurity/MagiskTrustUserCerts

# 3. Reboot — the module copies user certs to system store at boot
```

### Android 14+ (APEX containers)

Android 14 moved the system CA store to APEX containers. Use a module
that handles this:

- **Cert-Fixer** (houyidg/cert-fixer)
- **TrustAnyCert** (gorkemgun/TrustAnyCert)
- **Custom Certificate Authorities** (Magisk-Modules-Alt-Repo/custom-certificate-authorities)

### Manual installation (emulator)

```bash
# Start emulator with writable system
emulator -avd <avd_name> -writable-system
adb root
adb remount

# Get certificate hash
openssl x509 -inform PEM -subject_hash_old -in madhyamas-ca.pem | head -1

# Push certificate
adb push <hash>.0 /system/etc/security/cacerts/
adb shell chmod 644 /system/etc/security/cacerts/<hash>.0
adb reboot
```

## Approach 6: Flutter Apps

**Challenge**: Flutter uses BoringSSL embedded in `libflutter.so`, bypassing
all Java-level hooks and the system trust store.

### reFlutter (static patching, no root)

```bash
pip install reflutter
reflutter target-app.apk
# Output: target-app.reflutter.apk
adb install target-app.reflutter.apk
```

reFlutter patches `libflutter.so` to disable SSL verification. No root
required, but only works for supported Flutter engine versions.

### Frida native hooking (root)

```javascript
// Frida script to bypass Flutter SSL pinning
var flutter = Process.getModuleByName("libflutter.so");
// Pattern for ssl_verify_cert_chain on ARM64
var sig = "55 41 57 41 56 41 55 41 54 53 48 83 EC 38 C6 02";
Memory.scan(flutter.base, flutter.size, sig, {
    onMatch: function(addr) {
        Interceptor.attach(addr, {
            onLeave: function(retval) {
                retval.replace(0x1); // Force SSL_VERIFY_OK
            }
        });
    }
});
```

### Automated tools

- **universal-flutter-ssl-pinning** (vichhka-git/universal-flutter-ssl-pinning)
  - Uses PyGhidra for static analysis
  - Auto-generates Frida scripts
- **flutter-ssl-pinning-bypass** (s4nsec/flutter-ssl-pinning-bypass)
  - Analyzes APK for Flutter version
  - Generates Frida hooks

## Approach 7: React Native Apps

**Works with**: Standard OkHttp bypass (most RN apps use OkHttp)

### Frida script for React Native

```javascript
Java.perform(function() {
    // OkHttp 3.x/4.x (most RN apps)
    try {
        var CP = Java.use('okhttp3.CertificatePinner');
        CP.check.overload('java.lang.String', 'java.util.List')
            .implementation = function() {};
    } catch(e) {}

    // Internal OkHttp (some RN apps)
    try {
        var CP2 = Java.use('com.android.okhttp.CertificatePinner');
        CP2.check.overload('java.lang.String', 'java.util.List')
            .implementation = function() {};
    } catch(e) {}

    // Custom TrustManager
    try {
        var TrustManager = Java.registerClass({
            implements: [Java.use('javax.net.ssl.X509TrustManager')],
            methods: {
                checkClientTrusted: function() {},
                checkServerTrusted: function() {},
                getAcceptedIssuers: function() { return []; }
            }
        });
    } catch(e) {}
});
```

## Decision Matrix

| Scenario | Recommended Approach | Root? |
|----------|---------------------|-------|
| No pinning | Install CA (VPN app or system) | No |
| NSC pinning only | APK patching (apk-mitm) | No |
| OkHttp pinning | apk-mitm or Frida | No / Yes |
| Custom TrustManager | Frida or LSPosed | Yes |
| Flutter app | reFlutter or Frida native | No / Yes |
| React Native app | Frida (OkHttp hook) | Yes |
| Cronet-based app | LSPosed (TrustMe) | Yes |
| Anti-Frida detection | phantom-frida + Frida | Yes |
| Multiple apps | LSPosed (system-wide) | Yes |
| Non-rooted, any app | apk-mitm + Frida gadget | No |

## Combining with Madhyamas VPN

The Madhyamas VPN companion app handles **traffic routing**. For pinning
bypass, use the approaches above in combination:

1. **Start Madhyamas proxy** on your computer
2. **Install Madhyamas VPN app** on the Android device
3. **Install CA certificate** via the VPN app
4. **Apply pinning bypass** (one of the approaches above)
5. **Start the VPN** — traffic flows: App → VPN → Madhyamas proxy

The VPN eliminates the need for manual proxy configuration in WiFi
settings and allows per-app interception.
