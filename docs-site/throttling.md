# Throttling

Throttling lets you **simulate slow or unreliable network conditions** so you can test how your app behaves under poor connectivity. Slow down downloads, add latency, or randomly drop requests to reproduce real-world network issues.

![Throttle View](/screenshots/throttle-view.png)

## How Throttling Works

When throttling is enabled, Madhyamas delays or limits traffic passing through the proxy according to your settings. The traffic still reaches its destination — just slower. This helps you:

- Test loading states and progress indicators
- Verify timeout handling
- Reproduce issues that only happen on slow connections
- Test offline behavior and retry logic

## Throttle Settings

| Setting | Description |
|---------|-------------|
| **Download Speed** | Maximum download speed in bytes per second (e.g., 50000 = 50 KB/s) |
| **Upload Speed** | Maximum upload speed in bytes per second |
| **Latency** | Added delay before each request in milliseconds |
| **Packet Loss** | Percentage of requests to randomly drop (0–100%) |

## Throttle Presets

Madhyamas includes common network profiles so you don't have to configure settings manually:

| Preset | Download | Upload | Latency | Use Case |
|--------|----------|--------|---------|----------|
| **No throttling** | Unlimited | Unlimited | 0ms | Normal conditions |
| **Slow 3G** | 50 KB/s | 20 KB/s | 200ms | Mobile edge networks |
| **Fast 3G** | 180 KB/s | 84 KB/s | 150ms | 3G mobile |
| **Regular 4G** | 4 MB/s | 3 MB/s | 50ms | Standard 4G LTE |
| **Slow Wi-Fi** | 500 KB/s | 500 KB/s | 20ms | Poor Wi-Fi |
| **Offline** | 0 | 0 | — | No connectivity |

To apply a preset, click it in the presets list. The settings update automatically.

## Enabling and Disabling

Throttling is off by default. To turn it on:

1. Navigate to the **Throttle** view
2. Choose a preset or configure custom settings
3. Toggle the **Enabled** switch

When enabled, all traffic through the proxy is affected. Toggle it off to return to normal speed.

## Common Use Cases

### Testing Loading Indicators

Set throttle to "Slow 3G" and load your app. Verify that loading spinners, skeleton screens, and progress bars appear and behave correctly.

### Testing Timeouts

Set a high latency (e.g., 5000ms) and verify that your app's HTTP client times out gracefully instead of hanging indefinitely.

### Testing Retry Logic

Enable packet loss (e.g., 30%) and verify that your app retries failed requests with exponential backoff.

### Testing Offline Mode

Use the "Offline" preset to verify that your app shows appropriate offline messages and caches data correctly.

### Reproducing Mobile Issues

Use "Slow 3G" or "Fast 3G" presets to reproduce bugs that only appear on mobile networks but not on your fast development machine.
