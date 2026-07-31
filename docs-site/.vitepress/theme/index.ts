import { h } from "vue"
import DefaultTheme from "vitepress/theme"
import type { Theme } from "vitepress"
import { useData } from "vitepress"
import HomeLayout from "./components/HomeLayout.vue"
import "./style.css"

export default {
  extends: DefaultTheme,
  Layout: () => {
    // Check if this page uses the custom home layout.
    // When it does, render the full-page landing without the VitePress
    // chrome (nav bar, sidebar, footer) so it looks like a standalone site.
    const { frontmatter } = useData()
    if (frontmatter.value.layout === "home-layout") {
      return h(HomeLayout)
    }
    return h(DefaultTheme.Layout)
  },
  enhanceApp({ app }) {
    app.component("home-layout", HomeLayout)
  },
} satisfies Theme
