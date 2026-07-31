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
        ],
      },
      {
        text: "Inspecting Traffic",
        items: [
          { text: "Traffic Inspection", link: "/traffic-inspection" },
          { text: "Sessions", link: "/sessions" },
        ],
      },
      {
        text: "Modifying Traffic",
        items: [
          { text: "Breakpoints", link: "/breakpoints" },
          { text: "Mocks", link: "/mocks" },
          { text: "Rewrites", link: "/rewrites" },
          { text: "Throttling", link: "/throttling" },
          { text: "Replay", link: "/replay" },
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
