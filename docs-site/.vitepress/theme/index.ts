import { h } from "vue"
import DefaultTheme from "vitepress/theme"
import type { Theme } from "vitepress"
import "./style.css"

export default {
  extends: DefaultTheme,
  Layout: () => {
    return h(DefaultTheme.Layout, null, {
      // Additional slots can be added here if needed
    })
  },
  enhanceApp({ app }) {
    // Custom theme overrides via CSS variables
  },
} satisfies Theme
