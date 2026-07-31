import { h } from "vue"
import DefaultTheme from "vitepress/theme"
import type { Theme } from "vitepress"
import HomeLayout from "./components/HomeLayout.vue"
import "./style.css"

export default {
  extends: DefaultTheme,
  Layout: () => {
    return h(DefaultTheme.Layout, null, {
      // Additional slots can be added here if needed
    })
  },
  enhanceApp({ app }) {
    // Register custom layout for the landing page
    app.component("home-layout", HomeLayout)
  },
} satisfies Theme
