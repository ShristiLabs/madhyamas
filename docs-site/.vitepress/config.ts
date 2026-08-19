import { defineConfig } from "vitepress"

export default defineConfig({
  title: "Madhyamas",
  description:
    "Madhyamas is a free, open-source HTTP/HTTPS debugging proxy with a modern web UI — capture, inspect, mock, and replay traffic on Linux, macOS, Windows, and ARM.",

  // GitHub Pages serves at /madhyamas/ for the ShristiLabs/madhyamas repo
  base: "/madhyamas/",

  lang: "en-US",
  cleanUrls: true,

  // The repo README for the docs site itself is contributor-facing; don't
  // publish it as an unlisted page.
  srcExclude: ["README.md"],

  // Show last-modified time on each page (Git commit date of the file)
  lastUpdated: true,

  // Sitemap for search engines (emitted to .vitepress/dist/sitemap.xml)
  // Include the base path so URLs resolve correctly on GitHub Pages.
  sitemap: {
    hostname: "https://shristilabs.github.io/madhyamas/",
  },

  // Ignore dead links to localhost URLs (used in setup instructions)
  ignoreDeadLinks: [
    /^https?:\/\/localhost/,
    /^https?:\/\/127\.0\.0\.1/,
    /^https?:\/\/<your-computer-ip>/,
  ],

  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: "/madhyamas/favicon.svg" }],
    ["meta", { name: "theme-color", content: "#0b0e14" }],
    // Open Graph / Twitter cards so shared links render properly in chat apps
    ["meta", { property: "og:site_name", content: "Madhyamas" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:url", content: "https://shristilabs.github.io/madhyamas/" }],
    ["meta", { property: "og:title", content: "Madhyamas — Open-source HTTP/HTTPS debugging proxy" }],
    [
      "meta",
      {
        property: "og:description",
        content:
          "A free, open-source HTTP/HTTPS debugging proxy with a modern web UI. Capture, inspect, mock, and replay traffic on Linux, macOS, Windows, and ARM.",
      },
    ],
    ["meta", { property: "og:image", content: "https://shristilabs.github.io/madhyamas/screenshots/app-overview.png" }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:title", content: "Madhyamas — Open-source HTTP/HTTPS debugging proxy" }],
    [
      "meta",
      {
        name: "twitter:description",
        content:
          "A free, open-source HTTP/HTTPS debugging proxy with a modern web UI. Capture, inspect, mock, and replay traffic on Linux, macOS, Windows, and ARM.",
      },
    ],
    ["meta", { name: "twitter:image", content: "https://shristilabs.github.io/madhyamas/screenshots/app-overview.png" }],
  ],

  themeConfig: {
    logo: "/favicon.svg",

    nav: [
      { text: "Guides", link: "/getting-started" },
      { text: "Use Cases", link: "/use-cases" },
      { text: "Features", link: "/traffic-inspection" },
      { text: "Enterprise", link: "/enterprise/" },
      { text: "Reference", link: "/cli" },
      { text: "AI Agents", link: "/mcp" },
      { text: "GitHub", link: "https://github.com/ShristiLabs/madhyamas" },
    ],

    sidebar: [
      {
        text: "Getting Started",
        items: [
          { text: "Overview", link: "/" },
          { text: "Installation & Setup", link: "/getting-started" },
          { text: "Use Cases", link: "/use-cases" },
          { text: "HTTPS & Certificates", link: "/https-certificates" },
          { text: "Mobile Setup", link: "/mobile-setup" },
          { text: "Configuration", link: "/configuration" },
          { text: "Recording Limits", link: "/recording-limits" },
          { text: "Access Control", link: "/access-control" },
        ],
      },
      {
        text: "Inspecting Traffic",
        items: [
          { text: "Traffic Inspection", link: "/traffic-inspection" },
          { text: "Timeline View", link: "/timeline-view" },
          { text: "Focus", link: "/focus" },
          { text: "Sessions", link: "/sessions" },
          { text: "Importing HAR Files", link: "/har-import" },
        ],
      },
      {
        text: "Modifying Traffic",
        items: [
          { text: "Breakpoints", link: "/breakpoints" },
          { text: "Mocks", link: "/mocks" },
          { text: "Rewrites", link: "/rewrites" },
          { text: "Rewrite Templates", link: "/rewrite-templates" },
          { text: "Block List", link: "/block-list" },
          { text: "Throttling", link: "/throttling" },
          { text: "Replay", link: "/replay" },
        ],
      },
      {
        text: "Proxy & Networking",
        items: [
          { text: "SOCKS5 Proxy", link: "/socks-proxy" },
          { text: "Upstream Proxy", link: "/upstream-proxy" },
          { text: "HTTP/2 & gRPC", link: "/http2-grpc" },
          { text: "WebSocket Inspection", link: "/websockets" },
        ],
      },
      {
        text: "Automation & Extensibility",
        items: [
          { text: "Scripting", link: "/scripting" },
          { text: "Plugins", link: "/plugins" },
        ],
      },
      {
        text: "Tools",
        items: [
          { text: "Auto Save", link: "/auto-save" },
          { text: "Mirror", link: "/mirror" },
          { text: "Logging & Log Rotation", link: "/logging" },
        ],
      },
      {
        text: "Enterprise",
        items: [
          { text: "Overview", link: "/enterprise/" },
          { text: "Getting Started", link: "/enterprise/getting-started" },
          { text: "Authentication", link: "/enterprise/authentication" },
          { text: "User Management", link: "/enterprise/user-management" },
          { text: "Role-Based Access Control", link: "/enterprise/rbac" },
          { text: "Audit Logging", link: "/enterprise/audit-logging" },
          { text: "Performance & Monitoring", link: "/enterprise/monitoring" },
          { text: "Licensing", link: "/enterprise/licensing" },
          { text: "Multi-Instance Deployment", link: "/enterprise/deployment" },
          { text: "Configuration", link: "/enterprise/configuration" },
          { text: "CLI & MCP Tools", link: "/enterprise/cli-mcp" },
        ],
      },
      {
        text: "Reference",
        items: [
          { text: "CLI Reference", link: "/cli" },
          { text: "REST API Reference", link: "/rest-api" },
          { text: "MCP & AI Agents", link: "/mcp" },
          { text: "Security Overview", link: "/security" },
          { text: "Migrating from Charles Proxy", link: "/migration-from-charles" },
          { text: "Troubleshooting", link: "/troubleshooting" },
        ],
      },
    ],

    socialLinks: [
      { icon: "github", link: "https://github.com/ShristiLabs/madhyamas" },
    ],

    search: {
      provider: "local",
    },

    footer: {
      message: "Released under the MIT OR Apache-2.0 License.",
      copyright: "Copyright © 2024-present ShristiLabs",
    },

    outline: {
      label: "On this page",
      level: [2, 3],
    },

    docFooter: {
      prev: "Previous",
      next: "Next",
    },

    darkModeSwitchLabel: "Theme",
    sidebarMenuLabel: "Menu",
    returnToTopLabel: "Back to top",

    lastUpdatedText: "Last updated",

    editLink: {
      pattern: "https://github.com/ShristiLabs/madhyamas/edit/main/docs-site/:path",
      text: "Edit this page on GitHub",
    },
  },
})
