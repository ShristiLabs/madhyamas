import { defineConfig } from "vitepress"

export default defineConfig({
  title: "Madhyamas",
  description: "HTTP/HTTPS Debugging Proxy — User Documentation",

  // GitHub Pages serves at /madhyamas/ for the ShristiLabs/madhyamas repo
  base: "/madhyamas/",

  lang: "en-US",
  cleanUrls: true,

  // Ignore dead links to localhost URLs (used in setup instructions)
  ignoreDeadLinks: [
    /^https?:\/\/localhost/,
    /^https?:\/\/127\.0\.0\.1/,
    /^https?:\/\/<your-computer-ip>/,
  ],

  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: "/madhyamas/favicon.svg" }],
    ["meta", { name: "theme-color", content: "#0b0e14" }],
  ],

  themeConfig: {
    logo: "/favicon.svg",

    nav: [
      { text: "Guides", link: "/getting-started" },
      { text: "Features", link: "/traffic-inspection" },
      { text: "GitHub", link: "https://github.com/ShristiLabs/madhyamas" },
    ],

    sidebar: [
      {
        text: "Getting Started",
        items: [
          { text: "Overview", link: "/" },
          { text: "Installation & Setup", link: "/getting-started" },
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
  },
})
