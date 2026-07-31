<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue"

const year = ref(new Date().getFullYear())
const mobileOpen = ref(false)
const scrolled = ref(false)

function onScroll() {
  scrolled.value = window.scrollY > 8
}

function toggleMobile() {
  mobileOpen.value = !mobileOpen.value
}

function closeMobile() {
  mobileOpen.value = false
}

onMounted(() => {
  window.addEventListener("scroll", onScroll, { passive: true })

  // Reveal-on-scroll animation
  if ("IntersectionObserver" in window) {
    const revealEls = document.querySelectorAll(
      ".lp-card, .lp-mini, .lp-featurelist li, .lp-callout, .lp-codecard, .lp-terminal, .lp-chat, .lp-compare tbody tr, .lp-install__col, .lp-quickstart"
    )
    revealEls.forEach((el) => el.classList.add("lp-reveal"))

    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add("in")
            io.unobserve(entry.target)
          }
        })
      },
      { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
    )
    revealEls.forEach((el) => io.observe(el))
  }
})

onUnmounted(() => {
  window.removeEventListener("scroll", onScroll)
})
</script>

<template>
  <div class="lp">
    <!-- ===================== NAV ===================== -->
    <header class="lp-nav" :class="{ scrolled }">
      <div class="lp-container lp-nav__inner">
        <a href="#top" class="lp-brand" aria-label="Madhyamas home">
          <span class="lp-brand__mark" aria-hidden="true">
            <svg viewBox="0 0 32 32" width="28" height="28"><rect width="32" height="32" rx="7" fill="currentColor"/><path d="M9 22V10l7 8 7-8v12" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </span>
          <span class="lp-brand__name">Madhyamas</span>
        </a>

        <nav class="lp-nav__links" aria-label="Primary">
          <a href="#features">Features</a>
          <a href="#inspect">Inspection</a>
          <a href="#manipulate">Manipulation</a>
          <a href="#compare">Compare</a>
          <a href="#ai">AI Agents</a>
          <a href="#install">Install</a>
        </nav>

        <div class="lp-nav__actions">
          <a class="lp-btn lp-btn--ghost" href="https://github.com/ShristiLabs/madhyamas" target="_blank" rel="noopener">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .5C5.7.5.5 5.7.5 12c0 5.1 3.3 9.4 7.9 10.9.6.1.8-.2.8-.5v-1.8c-3.2.7-3.9-1.5-3.9-1.5-.5-1.3-1.3-1.7-1.3-1.7-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.7 1.3 3.4 1 .1-.8.4-1.3.7-1.6-2.6-.3-5.3-1.3-5.3-5.7 0-1.3.5-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.1 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0C17 4.7 18 5 18 5c.6 1.6.2 2.8.1 3.1.8.8 1.2 1.8 1.2 3.1 0 4.4-2.7 5.4-5.3 5.7.4.4.8 1.1.8 2.2v3.3c0 .3.2.6.8.5 4.6-1.5 7.9-5.8 7.9-10.9C23.5 5.7 18.3.5 12 .5z"/></svg>
            <span>GitHub</span>
          </a>
          <a class="lp-btn lp-btn--primary" href="#install">Get Started</a>
          <button class="lp-nav__toggle" @click="toggleMobile" aria-label="Toggle menu" :aria-expanded="mobileOpen">
            <span></span><span></span><span></span>
          </button>
        </div>
      </div>
      <div class="lp-nav__mobile" v-show="mobileOpen">
        <a href="#features" @click="closeMobile">Features</a>
        <a href="#inspect" @click="closeMobile">Inspection</a>
        <a href="#manipulate" @click="closeMobile">Manipulation</a>
        <a href="#compare" @click="closeMobile">Compare</a>
        <a href="#ai" @click="closeMobile">AI Agents</a>
        <a href="#install" @click="closeMobile">Install</a>
        <a class="lp-btn lp-btn--primary" href="https://github.com/ShristiLabs/madhyamas" target="_blank" rel="noopener">View on GitHub</a>
      </div>
    </header>

    <main id="top">
      <!-- ===================== HERO ===================== -->
      <section class="lp-hero">
        <div class="lp-hero__bg" aria-hidden="true">
          <div class="lp-hero__glow lp-hero__glow--1"></div>
          <div class="lp-hero__glow lp-hero__glow--2"></div>
          <div class="lp-hero__grid"></div>
        </div>
        <div class="lp-container lp-hero__inner">
          <div class="lp-hero__content">
            <span class="lp-pill">
              <span class="lp-pill__dot"></span> Open Source &middot; Rust-powered &middot; MIT / Apache-2.0
            </span>
            <h1 class="lp-hero__title">
              See every byte.<br />
              <span class="lp-grad">Debug any HTTP traffic.</span>
            </h1>
            <p class="lp-hero__lead">
              Madhyamas is a high-performance, cross-platform HTTP/HTTPS debugging proxy
              with a modern web UI. The free, open-source alternative to Charles Proxy and Fiddler.
            </p>
            <div class="lp-hero__cta">
              <a class="lp-btn lp-btn--primary lp-btn--lg" href="./getting-started">
                Get Started
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M5 12h14M13 6l6 6-6 6" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </a>
              <a class="lp-btn lp-btn--ghost lp-btn--lg" href="https://github.com/ShristiLabs/madhyamas" target="_blank" rel="noopener">
                View on GitHub
              </a>
            </div>
            <ul class="lp-hero__stats">
              <li><strong>Rust</strong><span>Memory-safe core</span></li>
              <li><strong>Web UI</strong><span>Browser-based</span></li>
              <li><strong>8 platforms</strong><span>Linux &middot; macOS &middot; Windows &middot; ARM</span></li>
            </ul>
          </div>

          <!-- Terminal mockup -->
          <div class="lp-terminal" aria-hidden="true">
            <div class="lp-terminal__bar">
              <span class="lp-terminal__dot lp-terminal__dot--r"></span>
              <span class="lp-terminal__dot lp-terminal__dot--y"></span>
              <span class="lp-terminal__dot lp-terminal__dot--g"></span>
              <span class="lp-terminal__title">madhyamas — traffic</span>
            </div>
            <div class="lp-terminal__body">
<pre><span class="t-dim">$</span> madhyamas
<span class="t-ok">✓</span> Proxy listening on <span class="t-blue">:8888</span>
<span class="t-ok">✓</span> Web UI at <span class="t-blue">http://localhost:3001</span>
<span class="t-ok">✓</span> CA cert ready for HTTPS interception

<span class="t-dim">#  method  status  host              path              ms</span>
<span class="t-num">1</span>  GET     <span class="t-green">200</span>    api.example.com   /v1/users         <span class="t-dim">42</span>
<span class="t-num">2</span>  POST    <span class="t-green">201</span>    api.example.com   /v1/orders        <span class="t-dim">118</span>
<span class="t-num">3</span>  GET     <span class="t-red">502</span>    pinned.app        /api/checkout     <span class="t-dim">--</span>
<span class="t-num">4</span>  PUT     <span class="t-green">204</span>    cdn.example.com   /assets/img.png   <span class="t-dim">9</span>
<span class="t-num">5</span>  GET     <span class="t-yellow">304</span>    api.example.com   /v1/users         <span class="t-dim">12</span>

<span class="t-dim">→ Live capture running · WebSocket streaming</span></pre>
            </div>
          </div>
        </div>
      </section>

      <!-- ===================== TRUST BAR ===================== -->
      <section class="lp-trust">
        <div class="lp-container lp-trust__inner">
          <span class="lp-trust__label">Built with</span>
          <div class="lp-trust__logos">
            <span>Rust</span><span>axum</span><span>hyper</span><span>tokio</span><span>rustls</span><span>React</span><span>TypeScript</span><span>Vite</span>
          </div>
        </div>
      </section>

      <!-- ===================== FEATURES ===================== -->
      <section class="lp-section" id="features">
        <div class="lp-container">
          <header class="lp-section__head">
            <span class="lp-eyebrow">Core capabilities</span>
            <h2>Everything you need to inspect traffic</h2>
            <p class="lp-section__sub">A complete debugging toolkit that captures, inspects, and manipulates HTTP, HTTPS, WebSocket, and gRPC traffic in real time.</p>
          </header>

          <div class="lp-grid lp-grid--3">
            <article class="lp-card">
              <div class="lp-card__icon lp-card__icon--blue">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h4l3-8 4 16 3-8h4" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </div>
              <h3>HTTP/HTTPS Interception</h3>
              <p>Capture and inspect all HTTP/HTTPS traffic in real time with automatic on-the-fly TLS certificate generation.</p>
            </article>

            <article class="lp-card">
              <div class="lp-card__icon lp-card__icon--cyan">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="14" rx="2"/><path d="M3 10h18M8 21h8" stroke-linecap="round"/></svg>
              </div>
              <h3>Modern Web UI</h3>
              <p>A React-based interface with real-time WebSocket streaming — no polling, no refresh, just live traffic.</p>
            </article>

            <article class="lp-card">
              <div class="lp-card__icon lp-card__icon--violet">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 6h16M4 12h16M4 18h10" stroke-linecap="round"/><circle cx="19" cy="18" r="2.5"/></svg>
              </div>
              <h3>Smart Filtering</h3>
              <p>Filter by URL, method, status, host, content type, duration, headers, and cookies to find anything fast.</p>
            </article>

            <article class="lp-card">
              <div class="lp-card__icon lp-card__icon--green">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v6m0 8v6m10-10h-6M8 12H2" stroke-linecap="round"/><circle cx="12" cy="12" r="3"/></svg>
              </div>
              <h3>HTTP/2 Upstream</h3>
              <p>Full HTTP/2 support for upstream connections with ALPN negotiation out of the box.</p>
            </article>

            <article class="lp-card">
              <div class="lp-card__icon lp-card__icon--amber">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-9-9" stroke-linecap="round"/><path d="M21 4v5h-5" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </div>
              <h3>Request Replay</h3>
              <p>Re-execute captured requests with modifications. Save and replay without retyping a thing.</p>
            </article>

            <article class="lp-card">
              <div class="lp-card__icon lp-card__icon--pink">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h16v6H4zM4 14h16v6H4z" stroke-linejoin="round"/><circle cx="8" cy="7" r="1" fill="currentColor"/><circle cx="8" cy="17" r="1" fill="currentColor"/></svg>
              </div>
              <h3>Session Management</h3>
              <p>Save, load, and share debugging sessions. Export to HAR, import previous captures, switch contexts instantly.</p>
            </article>
          </div>
        </div>
      </section>

      <!-- ===================== INSPECTION ===================== -->
      <section class="lp-section lp-section--alt" id="inspect">
        <div class="lp-container">
          <header class="lp-section__head">
            <span class="lp-eyebrow">Traffic inspection</span>
            <h2>Read bodies like a pro</h2>
            <p class="lp-section__sub">Syntax-highlighted JSON, image previews, decompression, and powerful query languages built in.</p>
          </header>

          <div class="lp-split">
            <div class="lp-split__media" aria-hidden="true">
              <div class="lp-codecard">
                <div class="lp-codecard__tabs">
                  <span class="lp-codecard__tab lp-codecard__tab--active">Tree</span>
                  <span class="lp-codecard__tab">Code</span>
                  <span class="lp-codecard__chip">JSONPath</span>
                </div>
                <pre class="lp-codecard__body"><span class="c-key">{</span>
  <span class="c-prop">"store"</span>: {
    <span class="c-prop">"book"</span>: [
      { <span class="c-prop">"title"</span>: <span class="c-str">"Say Nothing"</span>, <span class="c-prop">"price"</span>: <span class="c-num">8.95</span> },
      { <span class="c-prop">"title"</span>: <span class="c-str">"The Pragmatic Programmer"</span>, <span class="c-prop">"price"</span>: <span class="c-num">29.99</span> }
    ]
  }
<span class="c-key">}</span>

<span class="c-dim">// $.store.book[*].title</span>
<span class="c-str">→ ["Say Nothing", "The Pragmatic Programmer"]</span></pre>
              </div>
            </div>
            <ul class="lp-featurelist">
              <li>
                <span class="lp-featurelist__icon">
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 6h16M4 12h10M4 18h7" stroke-linecap="round"/></svg>
                </span>
                <div>
                  <h4>Syntax-highlighted JSON viewer</h4>
                  <p>Prism.js-powered highlighting with collapsible Tree and Code views, plus prettify / minify toggle.</p>
                </div>
              </li>
              <li>
                <span class="lp-featurelist__icon">
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4a7 7 0 1 1 0 14 7 7 0 0 1 0-14zm0 3v8m-4-4h8" stroke-linecap="round"/></svg>
                </span>
                <div>
                  <h4>JSONPath &amp; JMESPath queries</h4>
                  <p>Filter and extract JSON data with expressions like <code>$.store.book[*].title</code>.</p>
                </div>
              </li>
              <li>
                <span class="lp-featurelist__icon">
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="3"/><circle cx="9" cy="9" r="2"/><path d="m21 15-5-5L5 21" stroke-linecap="round" stroke-linejoin="round"/></svg>
                </span>
                <div>
                  <h4>Image preview</h4>
                  <p>Automatic rendering for PNG, JPEG, GIF, WebP, SVG, ICO, BMP, AVIF, and TIFF responses with download.</p>
                </div>
              </li>
              <li>
                <span class="lp-featurelist__icon">
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16M4 12h16M4 17h10" stroke-linecap="round"/></svg>
                </span>
                <div>
                  <h4>Decompression &amp; decoding</h4>
                  <p>Toggle gzip / deflate / brotli decompression on demand. Automatic base64 decoding of binary bodies.</p>
                </div>
              </li>
              <li>
                <span class="lp-featurelist__icon">
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" stroke-linejoin="round"/></svg>
                </span>
                <div>
                  <h4>Copy as cURL / HTTPie / fetch / wget</h4>
                  <p>Export any request as a ready-to-paste command-line command in your favorite tool.</p>
                </div>
              </li>
            </ul>
          </div>
        </div>
      </section>

      <!-- ===================== MANIPULATION ===================== -->
      <section class="lp-section" id="manipulate">
        <div class="lp-container">
          <header class="lp-section__head">
            <span class="lp-eyebrow">Traffic manipulation</span>
            <h2>Don't just watch — take control</h2>
            <p class="lp-section__sub">Pause, mock, rewrite, throttle, and replay traffic to reproduce any scenario.</p>
          </header>

          <div class="lp-grid lp-grid--4">
            <article class="lp-mini">
              <div class="lp-mini__icon lp-mini__icon--blue">
                <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18V5l12-2v13" stroke-linejoin="round"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>
              </div>
              <h4>Breakpoints</h4>
              <p>Pause requests or responses for inspection and modification before forwarding.</p>
            </article>
            <article class="lp-mini">
              <div class="lp-mini__icon lp-mini__icon--green">
                <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16v10H4z" stroke-linejoin="round"/><path d="M8 12h8" stroke-linecap="round"/></svg>
              </div>
              <h4>Response Mocking</h4>
              <p>Serve custom responses instead of hitting real servers. Collections, recording, import &amp; export.</p>
            </article>
            <article class="lp-mini">
              <div class="lp-mini__icon lp-mini__icon--violet">
                <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h6M14 7h6M4 17h6M14 17h6" stroke-linecap="round"/><path d="M10 7l4 10" stroke-linecap="round"/></svg>
              </div>
              <h4>URL / Header Rewriting</h4>
              <p>Automatically modify traffic based on rules — redirect, replace, inject headers.</p>
            </article>
            <article class="lp-mini">
              <div class="lp-mini__icon lp-mini__icon--amber">
                <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12a9 9 0 0 1 9-9 9 9 0 0 1 8 5M21 12a9 9 0 0 1-9 9 9 9 0 0 1-8-5" stroke-linecap="round"/><path d="M16 8h5V3M8 16H3v5" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </div>
              <h4>Bandwidth Throttling</h4>
              <p>Simulate slow networks with 3G, 4G, and DSL presets to test under real conditions.</p>
            </article>
          </div>

          <div class="lp-callout">
            <div class="lp-callout__icon">
              <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 9v4m0 4h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </div>
            <div>
              <h4>SSL/TLS error visibility</h4>
              <p>Failed TLS handshakes (e.g. apps with certificate pinning) are recorded as 502 entries with a clear explanation of the cause — so nothing disappears silently.</p>
            </div>
          </div>
        </div>
      </section>

      <!-- ===================== ADVANCED ===================== -->
      <section class="lp-section lp-section--alt" id="advanced">
        <div class="lp-container">
          <header class="lp-section__head">
            <span class="lp-eyebrow">Advanced &amp; experimental</span>
            <h2>Beyond plain HTTP</h2>
            <p class="lp-section__sub">WebSocket capture, gRPC debugging, scripting, and a plugin system to extend Madhyamas your way.</p>
          </header>
          <div class="lp-grid lp-grid--3">
            <article class="lp-card lp-card--ghost">
              <div class="lp-card__icon lp-card__icon--cyan">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12c2-4 4-4 6 0s4 4 6 0 4-4 6 0" stroke-linecap="round"/></svg>
              </div>
              <h3>WebSocket Capture</h3>
              <p>Inspect WebSocket messages in real time alongside your HTTP traffic.</p>
            </article>
            <article class="lp-card lp-card--ghost">
              <div class="lp-card__icon lp-card__icon--violet">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z" stroke-linejoin="round"/></svg>
              </div>
              <h3>gRPC Support <span class="lp-tag lp-tag--beta">Experimental</span></h3>
              <p>Debug gRPC / Protocol Buffer traffic with frame parsing and stream inspection.</p>
            </article>
            <article class="lp-card lp-card--ghost">
              <div class="lp-card__icon lp-card__icon--amber">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="m8 6-6 6 6 6M16 6l6 6-6 6" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </div>
              <h3>JS/TS Scripting <span class="lp-tag lp-tag--beta">Experimental</span></h3>
              <p>Automate traffic manipulation with JavaScript and TypeScript scripts.</p>
            </article>
            <article class="lp-card lp-card--ghost">
              <div class="lp-card__icon lp-card__icon--green">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2 3 7v6c0 5 3.8 8.5 9 9 5.2-.5 9-4 9-9V7l-9-5z" stroke-linejoin="round"/></svg>
              </div>
              <h3>Plugin System <span class="lp-tag lp-tag--beta">Experimental</span></h3>
              <p>Extend functionality with custom Rust plugins. Enable, disable, and reload at runtime.</p>
            </article>
            <article class="lp-card lp-card--ghost">
              <div class="lp-card__icon lp-card__icon--pink">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v18M3 12h18" stroke-linecap="round"/><circle cx="12" cy="12" r="9"/></svg>
              </div>
              <h3>Enterprise Features <span class="lp-tag lp-tag--beta">Experimental</span></h3>
              <p>Authentication, user management, RBAC, audit logging, and an onboarding wizard.</p>
            </article>
            <article class="lp-card lp-card--ghost">
              <div class="lp-card__icon lp-card__icon--blue">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h16v12H4zM8 20h8M12 16v4" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </div>
              <h3>Android VPN Companion</h3>
              <p>A no-root Android app uses VpnService to transparently route device traffic to the proxy.</p>
            </article>
          </div>
        </div>
      </section>

      <!-- ===================== AI AGENTS ===================== -->
      <section class="lp-section" id="ai">
        <div class="lp-container">
          <div class="lp-ai">
            <div class="lp-ai__content">
              <span class="lp-eyebrow">AI agent integration</span>
              <h2>Debug with your AI assistant</h2>
              <p class="lp-section__sub">
                Madhyamas ships with a built-in MCP (Model Context Protocol) server. Let AI agents like Claude
                inspect traffic, create mocks, replay requests, and export sessions — directly from your conversation.
              </p>
              <ul class="lp-checklist">
                <li><span class="lp-check"></span> List, search, and inspect captured traffic</li>
                <li><span class="lp-check"></span> Create and toggle mock responses</li>
                <li><span class="lp-check"></span> Replay requests with modifications</li>
                <li><span class="lp-check"></span> Export sessions as HAR or cURL</li>
                <li><span class="lp-check"></span> Full CLI with <code>--json</code> for machine-readable output</li>
              </ul>
              <a class="lp-btn lp-btn--primary" href="https://github.com/ShristiLabs/madhyamas#mcp-server-for-ai-agents" target="_blank" rel="noopener">Read MCP docs</a>
            </div>
            <div class="lp-ai__media" aria-hidden="true">
              <div class="lp-chat">
                <div class="lp-chat__msg lp-chat__msg--user">Show me all failed requests to /api/users in the last 10 minutes</div>
                <div class="lp-chat__msg lp-chat__msg--ai">
                  <span class="lp-chat__who">Claude</span>
                  Found 3 failed requests to <code>/api/users</code>:
                  <ul>
                    <li><span class="t-red">502</span> — cert pinning failure</li>
                    <li><span class="t-red">500</span> — upstream timeout (8.2s)</li>
                    <li><span class="t-red">401</span> — expired token</li>
                  </ul>
                  Want me to mock a valid token and replay them?
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- ===================== COMPARISON ===================== -->
      <section class="lp-section lp-section--alt" id="compare">
        <div class="lp-container">
          <header class="lp-section__head">
            <span class="lp-eyebrow">How it compares</span>
            <h2>Why Madhyamas?</h2>
            <p class="lp-section__sub">Open source, free, cross-platform, Rust-powered, and built for the modern web.</p>
          </header>

          <div class="lp-tablewrap">
            <table class="lp-compare">
              <thead>
                <tr>
                  <th>Feature</th>
                  <th class="lp-compare__hl">Madhyamas</th>
                  <th>Charles</th>
                  <th>mitmproxy</th>
                  <th>Fiddler</th>
                  <th>Proxyman</th>
                </tr>
              </thead>
              <tbody>
                <tr><td>Open Source</td><td class="lp-compare__hl"><span class="yes">Yes</span></td><td><span class="no">No</span></td><td><span class="yes">Yes</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td></tr>
                <tr><td>Free</td><td class="lp-compare__hl"><span class="yes">Yes</span></td><td><span class="no">$50</span></td><td><span class="yes">Yes</span></td><td><span class="yes">Yes</span></td><td><span class="meh">Freemium</span></td></tr>
                <tr><td>Cross-platform</td><td class="lp-compare__hl"><span class="yes">Yes</span></td><td><span class="yes">Yes</span></td><td><span class="yes">Yes</span></td><td><span class="meh">Windows</span></td><td><span class="meh">macOS</span></td></tr>
                <tr><td>Web UI</td><td class="lp-compare__hl"><span class="yes">Yes</span></td><td><span class="no">No</span></td><td><span class="meh">Limited</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td></tr>
                <tr><td>Rust-powered</td><td class="lp-compare__hl"><span class="yes">Yes</span></td><td><span class="no">Java</span></td><td><span class="no">Python</span></td><td><span class="no">.NET</span></td><td><span class="no">Swift</span></td></tr>
                <tr><td>gRPC</td><td class="lp-compare__hl"><span class="meh">Exp.</span></td><td><span class="no">No</span></td><td><span class="yes">Yes</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td></tr>
                <tr><td>WebSocket</td><td class="lp-compare__hl"><span class="yes">Yes</span></td><td><span class="meh">Limited</span></td><td><span class="yes">Yes</span></td><td><span class="yes">Yes</span></td><td><span class="yes">Yes</span></td></tr>
                <tr><td>Scripting</td><td class="lp-compare__hl"><span class="meh">JS/TS</span></td><td><span class="no">No</span></td><td><span class="yes">Python</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td></tr>
                <tr><td>JSON Query</td><td class="lp-compare__hl"><span class="yes">JSONPath + JMESPath</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td></tr>
                <tr><td>MCP / AI Agent</td><td class="lp-compare__hl"><span class="yes">Yes</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td><td><span class="no">No</span></td></tr>
              </tbody>
            </table>
          </div>
        </div>
      </section>

      <!-- ===================== INSTALL ===================== -->
      <section class="lp-section" id="install">
        <div class="lp-container">
          <header class="lp-section__head">
            <span class="lp-eyebrow">Get started</span>
            <h2>Install in minutes</h2>
            <p class="lp-section__sub">A single unified binary includes the proxy, web UI, MCP server, and CLI. No runtime dependencies.</p>
          </header>

          <div class="lp-install">
            <div class="lp-install__col">
              <h3 class="lp-install__title">From source</h3>
              <div class="lp-terminal lp-terminal--sm">
                <div class="lp-terminal__bar"><span class="lp-terminal__title">build</span></div>
                <pre class="lp-terminal__body"><span class="t-dim">#</span> Clone &amp; build
git clone https://github.com/ShristiLabs/madhyamas.git
cd madhyamas
cargo build --release

<span class="t-dim">#</span> Run the proxy + web UI
./target/release/madhyamas</pre>
              </div>
            </div>

            <div class="lp-install__col">
              <h3 class="lp-install__title">Pre-built binaries</h3>
              <p class="lp-install__text">Download the latest release for your platform from the Releases page. Available for:</p>
              <div class="lp-platforms">
                <span class="lp-platform">Linux x86_64</span>
                <span class="lp-platform">Linux ARM64</span>
                <span class="lp-platform">Linux ARMv7</span>
                <span class="lp-platform">Linux ARMv6</span>
                <span class="lp-platform">Linux RISC-V</span>
                <span class="lp-platform">macOS Intel</span>
                <span class="lp-platform">macOS Apple Silicon</span>
                <span class="lp-platform">Windows x64</span>
              </div>
              <a class="lp-btn lp-btn--ghost" href="https://github.com/ShristiLabs/madhyamas/releases" target="_blank" rel="noopener">Download releases</a>
            </div>

            <div class="lp-install__col">
              <h3 class="lp-install__title">Subcommands</h3>
              <div class="lp-terminal lp-terminal--sm">
                <div class="lp-terminal__bar"><span class="lp-terminal__title">usage</span></div>
                <pre class="lp-terminal__body">madhyamas              <span class="t-dim"># proxy + web UI</span>
madhyamas serve        <span class="t-dim"># same as above</span>
madhyamas mcp          <span class="t-dim"># MCP server (stdio)</span>
madhyamas traffic list <span class="t-dim"># CLI command</span>
madhyamas --help       <span class="t-dim"># all commands</span></pre>
              </div>
            </div>
          </div>

          <div class="lp-quickstart">
            <h3>Quick start</h3>
            <ol>
              <li>Run <code>madhyamas</code> — proxy on <code>:8888</code>, web UI on <code>:3001</code></li>
              <li>Point your browser or app proxy to <code>localhost:8888</code></li>
              <li>Install the CA cert from <code>~/.madhyamas/certs/madhyamas-ca.pem</code> for HTTPS</li>
              <li>Open <code>http://localhost:3001</code> and start debugging</li>
            </ol>
          </div>
        </div>
      </section>

      <!-- ===================== DOCS CTA ===================== -->
      <section class="lp-section lp-section--alt" id="docs">
        <div class="lp-container">
          <header class="lp-section__head">
            <span class="lp-eyebrow">Documentation</span>
            <h2>Read the docs</h2>
            <p class="lp-section__sub">Step-by-step guides for every feature, from basic setup to advanced traffic manipulation.</p>
          </header>
          <div class="lp-grid lp-grid--3">
            <a class="lp-card lp-card--ghost" href="./getting-started">
              <div class="lp-card__icon lp-card__icon--blue">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M13 6l6 6-6 6" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </div>
              <h3>Getting Started</h3>
              <p>Installation, first launch, connecting your first client, and basic configuration.</p>
            </a>
            <a class="lp-card lp-card--ghost" href="./traffic-inspection">
              <div class="lp-card__icon lp-card__icon--cyan">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h4l3-8 4 16 3-8h4" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </div>
              <h3>Traffic Inspection</h3>
              <p>Viewing, filtering, searching, and exporting captured HTTP/HTTPS traffic.</p>
            </a>
            <a class="lp-card lp-card--ghost" href="./https-certificates">
              <div class="lp-card__icon lp-card__icon--violet">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4" stroke-linecap="round"/></svg>
              </div>
              <h3>HTTPS &amp; Certificates</h3>
              <p>Installing the Madhyamas CA certificate on macOS, Windows, Linux, iOS, and Android.</p>
            </a>
            <a class="lp-card lp-card--ghost" href="./breakpoints">
              <div class="lp-card__icon lp-card__icon--amber">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3" stroke-linecap="round"/></svg>
              </div>
              <h3>Breakpoints</h3>
              <p>Pause requests or responses, inspect them, and modify before forwarding.</p>
            </a>
            <a class="lp-card lp-card--ghost" href="./mocks">
              <div class="lp-card__icon lp-card__icon--green">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16v10H4z" stroke-linejoin="round"/><path d="M8 12h8" stroke-linecap="round"/></svg>
              </div>
              <h3>Mocks</h3>
              <p>Create fake API responses, record from real traffic, and organize into collections.</p>
            </a>
            <a class="lp-card lp-card--ghost" href="./mobile-setup">
              <div class="lp-card__icon lp-card__icon--pink">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><rect x="7" y="2" width="10" height="20" rx="2"/><path d="M11 18h2" stroke-linecap="round"/></svg>
              </div>
              <h3>Mobile Setup</h3>
              <p>Connect iPhones, iPads, and Android devices over Wi-Fi to debug mobile app traffic.</p>
            </a>
          </div>
        </div>
      </section>

      <!-- ===================== CTA ===================== -->
      <section class="lp-cta">
        <div class="lp-container lp-cta__inner">
          <h2>Start debugging smarter today</h2>
          <p>Free, open source, and built to last. Star the repo and join the community.</p>
          <div class="lp-cta__btns">
            <a class="lp-btn lp-btn--primary lp-btn--lg" href="https://github.com/ShristiLabs/madhyamas" target="_blank" rel="noopener">Star on GitHub</a>
            <a class="lp-btn lp-btn--ghost lp-btn--lg" href="https://github.com/ShristiLabs/madhyamas/issues" target="_blank" rel="noopener">Report an issue</a>
          </div>
        </div>
      </section>
    </main>

    <!-- ===================== FOOTER ===================== -->
    <footer class="lp-footer">
      <div class="lp-container lp-footer__inner">
        <div class="lp-footer__brand">
          <a href="#top" class="lp-brand">
            <span class="lp-brand__mark" aria-hidden="true">
              <svg viewBox="0 0 32 32" width="24" height="24"><rect width="32" height="32" rx="7" fill="currentColor"/><path d="M9 22V10l7 8 7-8v12" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </span>
            <span class="lp-brand__name">Madhyamas</span>
          </a>
          <p class="lp-footer__tag">Open source HTTP/HTTPS debugging proxy, powered by Rust.</p>
        </div>
        <div class="lp-footer__cols">
          <div class="lp-footer__col">
            <h5>Product</h5>
            <a href="#features">Features</a>
            <a href="#inspect">Inspection</a>
            <a href="#manipulate">Manipulation</a>
            <a href="#ai">AI Agents</a>
          </div>
          <div class="lp-footer__col">
            <h5>Docs</h5>
            <a href="./getting-started">Getting Started</a>
            <a href="./traffic-inspection">Traffic Inspection</a>
            <a href="./https-certificates">HTTPS &amp; Certificates</a>
            <a href="./mobile-setup">Mobile Setup</a>
          </div>
          <div class="lp-footer__col">
            <h5>Community</h5>
            <a href="https://github.com/ShristiLabs/madhyamas" target="_blank" rel="noopener">GitHub</a>
            <a href="https://github.com/ShristiLabs/madhyamas/issues" target="_blank" rel="noopener">Issues</a>
            <a href="https://github.com/ShristiLabs/madhyamas/discussions" target="_blank" rel="noopener">Discussions</a>
          </div>
        </div>
      </div>
      <div class="lp-container lp-footer__bottom">
        <span>Dual-licensed under MIT OR Apache-2.0.</span>
        <span>&copy; {{ year }} Madhyamas contributors.</span>
      </div>
    </footer>
  </div>
</template>

<style>
/* ============================================================
   Madhyamas — VitePress landing page
   All styles prefixed with lp- to avoid collisions with VitePress
   ============================================================ */

.lp {
  --lp-bg: #0b0e14;
  --lp-bg-soft: #0f131c;
  --lp-bg-elev: #141a26;
  --lp-surface: #161c2a;
  --lp-surface-2: #1b2233;
  --lp-border: #232c40;
  --lp-border-soft: #1a2233;
  --lp-text: #e6ebf5;
  --lp-text-soft: #aab3c5;
  --lp-text-dim: #6b7689;
  --lp-blue: #2563eb;
  --lp-blue-bright: #3b82f6;
  --lp-cyan: #06b6d4;
  --lp-violet: #8b5cf6;
  --lp-green: #22c55e;
  --lp-amber: #f59e0b;
  --lp-pink: #ec4899;
  --lp-red: #ef4444;
  --lp-yellow: #eab308;
  --lp-grad: linear-gradient(135deg, #3b82f6 0%, #06b6d4 50%, #8b5cf6 100%);
  --lp-radius: 14px;
  --lp-radius-sm: 10px;
  --lp-radius-lg: 22px;
  --lp-shadow: 0 1px 2px rgba(0,0,0,.4), 0 8px 24px rgba(0,0,0,.25);
  --lp-shadow-lg: 0 20px 60px rgba(0,0,0,.45);
  --lp-container: 1140px;
  --lp-ease: cubic-bezier(.4, 0, .2, 1);

  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background: var(--lp-bg);
  color: var(--lp-text);
  line-height: 1.6;
  font-size: 16px;
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
  overflow-x: hidden;
  min-height: 100vh;
}

.lp * { box-sizing: border-box; }
.lp img, .lp svg { display: block; max-width: 100%; }
.lp a { color: inherit; text-decoration: none; }
.lp ul { list-style: none; padding: 0; }
.lp code {
  font-family: ui-monospace, "SFMono-Regular", Menlo, Monaco, Consolas, monospace;
  background: rgba(37, 99, 235, .14);
  color: #bcd0ff;
  padding: .12em .42em;
  border-radius: 6px;
  font-size: .88em;
}
.lp pre { font-family: ui-monospace, "SFMono-Regular", Menlo, Monaco, Consolas, monospace; }

/* layout */
.lp-container { width: 100%; max-width: var(--lp-container); margin: 0 auto; padding: 0 24px; }
.lp-section { padding: 96px 0; }
.lp-section--alt { background: var(--lp-bg-soft); }
.lp-section__head { max-width: 680px; margin: 0 auto 56px; text-align: center; }
.lp-eyebrow {
  display: inline-block; font-size: .78rem; font-weight: 600;
  letter-spacing: .12em; text-transform: uppercase;
  color: var(--lp-blue-bright); margin-bottom: 14px;
}
.lp-section__head h2 {
  font-size: clamp(1.8rem, 4vw, 2.6rem); line-height: 1.15;
  letter-spacing: -.02em; font-weight: 700; margin: 0;
}
.lp-section__sub { margin-top: 16px; color: var(--lp-text-soft); font-size: 1.08rem; }

/* buttons */
.lp-btn {
  display: inline-flex; align-items: center; gap: 8px;
  padding: 10px 18px; border-radius: 10px;
  font-weight: 600; font-size: .95rem;
  border: 1px solid transparent; cursor: pointer;
  transition: transform .15s var(--lp-ease), background .2s var(--lp-ease), border-color .2s var(--lp-ease), box-shadow .2s var(--lp-ease);
  white-space: nowrap;
}
.lp-btn:hover { transform: translateY(-1px); }
.lp-btn:active { transform: translateY(0); }
.lp-btn--lg { padding: 13px 24px; font-size: 1rem; }
.lp-btn--primary { background: var(--lp-blue); color: #fff; box-shadow: 0 6px 20px rgba(37, 99, 235, .35); }
.lp-btn--primary:hover { background: var(--lp-blue-bright); box-shadow: 0 8px 26px rgba(59, 130, 246, .45); }
.lp-btn--ghost { background: rgba(255,255,255,.04); border-color: var(--lp-border); color: var(--lp-text); }
.lp-btn--ghost:hover { background: rgba(255,255,255,.08); border-color: #2f3a52; }

/* nav */
.lp-nav {
  position: sticky; top: 0; z-index: 50;
  background: rgba(11, 14, 20, .72);
  backdrop-filter: saturate(140%) blur(14px);
  -webkit-backdrop-filter: saturate(140%) blur(14px);
  border-bottom: 1px solid transparent;
  transition: border-color .25s var(--lp-ease), background .25s var(--lp-ease);
}
.lp-nav.scrolled { border-bottom-color: var(--lp-border); background: rgba(11, 14, 20, .9); }
.lp-nav__inner { display: flex; align-items: center; justify-content: space-between; height: 64px; gap: 20px; }
.lp-brand { display: inline-flex; align-items: center; gap: 10px; font-weight: 700; font-size: 1.1rem; letter-spacing: -.01em; }
.lp-brand__mark { color: var(--lp-blue); display: inline-flex; }
.lp-brand__name { color: var(--lp-text); }
.lp-nav__links { display: flex; gap: 26px; }
.lp-nav__links a { color: var(--lp-text-soft); font-size: .94rem; font-weight: 500; transition: color .15s var(--lp-ease); }
.lp-nav__links a:hover { color: var(--lp-text); }
.lp-nav__actions { display: flex; align-items: center; gap: 10px; }
.lp-nav__toggle {
  display: none; flex-direction: column; justify-content: center; gap: 5px;
  width: 40px; height: 40px; background: transparent;
  border: 1px solid var(--lp-border); border-radius: 10px; cursor: pointer;
}
.lp-nav__toggle span { display: block; height: 2px; width: 18px; margin: 0 auto; background: var(--lp-text); border-radius: 2px; transition: transform .2s var(--lp-ease), opacity .2s var(--lp-ease); }
.lp-nav__toggle[aria-expanded="true"] span:nth-child(1) { transform: translateY(7px) rotate(45deg); }
.lp-nav__toggle[aria-expanded="true"] span:nth-child(2) { opacity: 0; }
.lp-nav__toggle[aria-expanded="true"] span:nth-child(3) { transform: translateY(-7px) rotate(-45deg); }
.lp-nav__mobile {
  display: flex; flex-direction: column; gap: 4px;
  padding: 12px 24px 20px; border-bottom: 1px solid var(--lp-border); background: var(--lp-bg);
}
.lp-nav__mobile a { padding: 10px 0; color: var(--lp-text-soft); font-weight: 500; }
.lp-nav__mobile .lp-btn { margin-top: 8px; justify-content: center; }

/* hero */
.lp-hero { position: relative; padding: 88px 0 72px; overflow: hidden; }
.lp-hero__bg { position: absolute; inset: 0; z-index: 0; pointer-events: none; }
.lp-hero__grid {
  position: absolute; inset: 0;
  background-image:
    linear-gradient(to right, rgba(37,99,235,.06) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(37,99,235,.06) 1px, transparent 1px);
  background-size: 56px 56px;
  mask-image: radial-gradient(ellipse 80% 60% at 50% 0%, #000 30%, transparent 75%);
  -webkit-mask-image: radial-gradient(ellipse 80% 60% at 50% 0%, #000 30%, transparent 75%);
}
.lp-hero__glow { position: absolute; border-radius: 50%; filter: blur(90px); opacity: .5; }
.lp-hero__glow--1 { width: 520px; height: 520px; background: #1d4ed8; top: -180px; left: -120px; }
.lp-hero__glow--2 { width: 460px; height: 460px; background: #6d28d9; top: -120px; right: -100px; opacity: .35; }
.lp-hero__inner { position: relative; z-index: 1; display: grid; grid-template-columns: 1.05fr .95fr; gap: 56px; align-items: center; }
.lp-pill {
  display: inline-flex; align-items: center; gap: 8px;
  padding: 6px 14px; border: 1px solid var(--lp-border);
  border-radius: 999px; font-size: .82rem; color: var(--lp-text-soft);
  background: rgba(255,255,255,.03);
}
.lp-pill__dot { width: 8px; height: 8px; border-radius: 50%; background: var(--lp-green); box-shadow: 0 0 0 4px rgba(34,197,94,.18); }
.lp-hero__title { margin-top: 22px; font-size: clamp(2.4rem, 5.5vw, 3.8rem); line-height: 1.05; letter-spacing: -.03em; font-weight: 800; }
.lp-grad { background: var(--lp-grad); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; color: transparent; }
.lp-hero__lead { margin-top: 22px; font-size: 1.18rem; color: var(--lp-text-soft); max-width: 540px; }
.lp-hero__cta { margin-top: 32px; display: flex; flex-wrap: wrap; gap: 14px; }
.lp-hero__stats { margin-top: 44px; display: flex; flex-wrap: wrap; gap: 28px; }
.lp-hero__stats li { display: flex; flex-direction: column; }
.lp-hero__stats strong { font-size: 1.05rem; color: var(--lp-text); }
.lp-hero__stats span { font-size: .82rem; color: var(--lp-text-dim); }

/* terminal */
.lp-terminal { background: #0a0d13; border: 1px solid var(--lp-border); border-radius: var(--lp-radius); box-shadow: var(--lp-shadow-lg); overflow: hidden; }
.lp-terminal--sm { box-shadow: var(--lp-shadow); }
.lp-terminal__bar { display: flex; align-items: center; gap: 8px; padding: 12px 16px; background: #11151f; border-bottom: 1px solid var(--lp-border); }
.lp-terminal__dot { width: 12px; height: 12px; border-radius: 50%; }
.lp-terminal__dot--r { background: #ff5f57; }
.lp-terminal__dot--y { background: #febc2e; }
.lp-terminal__dot--g { background: #28c840; }
.lp-terminal__title { margin-left: 8px; color: var(--lp-text-dim); font-size: .8rem; font-family: ui-monospace, monospace; }
.lp-terminal__body { padding: 18px 20px; overflow-x: auto; }
.lp-terminal__body pre { font-size: .85rem; line-height: 1.7; color: #c8d2e3; margin: 0; }
.lp-terminal--sm .lp-terminal__body pre { font-size: .82rem; }
.t-dim { color: #5b6478; }
.t-blue { color: #60a5fa; }
.t-green { color: #4ade80; }
.t-red { color: #f87171; }
.t-yellow { color: #facc15; }
.t-ok { color: #4ade80; }
.t-num { color: #c084fc; }

/* trust bar */
.lp-trust { padding: 28px 0; border-top: 1px solid var(--lp-border-soft); border-bottom: 1px solid var(--lp-border-soft); background: var(--lp-bg); }
.lp-trust__inner { display: flex; align-items: center; gap: 24px; flex-wrap: wrap; }
.lp-trust__label { color: var(--lp-text-dim); font-size: .85rem; font-weight: 500; }
.lp-trust__logos { display: flex; flex-wrap: wrap; gap: 22px; }
.lp-trust__logos span { color: var(--lp-text-soft); font-weight: 600; font-size: .95rem; opacity: .8; }

/* grids & cards */
.lp-grid { display: grid; gap: 22px; }
.lp-grid--3 { grid-template-columns: repeat(3, 1fr); }
.lp-grid--4 { grid-template-columns: repeat(4, 1fr); }
.lp-card {
  background: var(--lp-surface); border: 1px solid var(--lp-border);
  border-radius: var(--lp-radius); padding: 26px;
  transition: transform .2s var(--lp-ease), border-color .2s var(--lp-ease), background .2s var(--lp-ease);
}
.lp-card:hover { transform: translateY(-3px); border-color: #2c3650; background: var(--lp-surface-2); }
.lp-card h3 { font-size: 1.12rem; margin: 16px 0 8px; font-weight: 650; }
.lp-card p { color: var(--lp-text-soft); font-size: .96rem; }
.lp-card--ghost { background: transparent; }
.lp-card--ghost:hover { background: rgba(255,255,255,.025); }
.lp-card__icon { width: 44px; height: 44px; border-radius: 12px; display: grid; place-items: center; color: #fff; }
.lp-card__icon--blue { background: linear-gradient(135deg, #2563eb, #1d4ed8); }
.lp-card__icon--cyan { background: linear-gradient(135deg, #06b6d4, #0891b2); }
.lp-card__icon--violet { background: linear-gradient(135deg, #8b5cf6, #6d28d9); }
.lp-card__icon--green { background: linear-gradient(135deg, #22c55e, #15803d); }
.lp-card__icon--amber { background: linear-gradient(135deg, #f59e0b, #d97706); }
.lp-card__icon--pink { background: linear-gradient(135deg, #ec4899, #be185d); }

/* mini cards */
.lp-mini { background: var(--lp-surface); border: 1px solid var(--lp-border); border-radius: var(--lp-radius-sm); padding: 22px; transition: transform .2s var(--lp-ease), border-color .2s var(--lp-ease); }
.lp-mini:hover { transform: translateY(-2px); border-color: #2c3650; }
.lp-mini h4 { margin: 14px 0 6px; font-size: 1rem; font-weight: 650; }
.lp-mini p { color: var(--lp-text-soft); font-size: .9rem; }
.lp-mini__icon { width: 38px; height: 38px; border-radius: 10px; display: grid; place-items: center; color: #fff; }
.lp-mini__icon--blue { background: linear-gradient(135deg, #2563eb, #1d4ed8); }
.lp-mini__icon--green { background: linear-gradient(135deg, #22c55e, #15803d); }
.lp-mini__icon--violet { background: linear-gradient(135deg, #8b5cf6, #6d28d9); }
.lp-mini__icon--amber { background: linear-gradient(135deg, #f59e0b, #d97706); }

/* tags */
.lp-tag { font-size: .68rem; font-weight: 600; padding: 2px 8px; border-radius: 999px; vertical-align: middle; margin-left: 4px; }
.lp-tag--beta { background: rgba(245, 158, 11, .16); color: #fbbf24; border: 1px solid rgba(245,158,11,.3); }

/* split */
.lp-split { display: grid; grid-template-columns: 1fr 1fr; gap: 48px; align-items: center; }
.lp-featurelist { display: flex; flex-direction: column; gap: 22px; }
.lp-featurelist li { display: flex; gap: 16px; }
.lp-featurelist__icon { flex: none; width: 40px; height: 40px; border-radius: 10px; background: rgba(37,99,235,.12); color: var(--lp-blue-bright); display: grid; place-items: center; }
.lp-featurelist h4 { font-size: 1.02rem; font-weight: 650; margin-bottom: 4px; }
.lp-featurelist p { color: var(--lp-text-soft); font-size: .94rem; }

/* code card */
.lp-codecard { background: #0a0d13; border: 1px solid var(--lp-border); border-radius: var(--lp-radius); box-shadow: var(--lp-shadow-lg); overflow: hidden; }
.lp-codecard__tabs { display: flex; align-items: center; gap: 4px; padding: 10px 14px; background: #11151f; border-bottom: 1px solid var(--lp-border); }
.lp-codecard__tab { padding: 6px 12px; border-radius: 8px; font-size: .82rem; color: var(--lp-text-dim); cursor: default; }
.lp-codecard__tab--active { background: rgba(37,99,235,.18); color: #bcd0ff; }
.lp-codecard__chip { margin-left: auto; font-size: .72rem; color: var(--lp-cyan); background: rgba(6,182,212,.12); padding: 4px 10px; border-radius: 999px; }
.lp-codecard__body { padding: 20px; overflow-x: auto; font-size: .85rem; line-height: 1.7; color: #c8d2e3; margin: 0; }
.c-key { color: #f87171; }
.c-prop { color: #60a5fa; }
.c-str { color: #4ade80; }
.c-num { color: #c084fc; }
.c-dim { color: #5b6478; }

/* callout */
.lp-callout { margin-top: 40px; display: flex; gap: 18px; align-items: flex-start; background: linear-gradient(135deg, rgba(245,158,11,.08), rgba(239,68,68,.06)); border: 1px solid rgba(245,158,11,.25); border-radius: var(--lp-radius); padding: 24px; }
.lp-callout__icon { flex: none; width: 44px; height: 44px; border-radius: 12px; background: rgba(245,158,11,.18); color: #fbbf24; display: grid; place-items: center; }
.lp-callout h4 { font-size: 1.05rem; margin-bottom: 4px; }
.lp-callout p { color: var(--lp-text-soft); font-size: .96rem; }

/* AI section */
.lp-ai { display: grid; grid-template-columns: 1fr 1fr; gap: 56px; align-items: center; }
.lp-checklist { display: flex; flex-direction: column; gap: 12px; margin: 24px 0 28px; }
.lp-checklist li { display: flex; align-items: center; gap: 12px; color: var(--lp-text-soft); }
.lp-check { flex: none; width: 22px; height: 22px; border-radius: 50%; background: rgba(34,197,94,.16); color: var(--lp-green); display: grid; place-items: center; }
.lp-check::after { content: ""; width: 10px; height: 6px; border-left: 2px solid currentColor; border-bottom: 2px solid currentColor; transform: rotate(-45deg) translate(1px, -1px); }
.lp-chat { background: #0a0d13; border: 1px solid var(--lp-border); border-radius: var(--lp-radius); padding: 22px; box-shadow: var(--lp-shadow-lg); display: flex; flex-direction: column; gap: 14px; }
.lp-chat__msg { padding: 14px 16px; border-radius: 12px; font-size: .92rem; line-height: 1.6; }
.lp-chat__msg--user { background: var(--lp-surface); border: 1px solid var(--lp-border); align-self: flex-start; max-width: 85%; }
.lp-chat__msg--ai { background: linear-gradient(135deg, rgba(37,99,235,.14), rgba(139,92,246,.1)); border: 1px solid rgba(37,99,235,.3); align-self: flex-end; max-width: 90%; }
.lp-chat__who { display: block; font-size: .75rem; font-weight: 600; color: var(--lp-blue-bright); margin-bottom: 6px; }
.lp-chat__msg ul { margin-top: 8px; display: flex; flex-direction: column; gap: 4px; }

/* comparison table */
.lp-tablewrap { overflow-x: auto; border: 1px solid var(--lp-border); border-radius: var(--lp-radius); background: var(--lp-surface); }
.lp-compare { width: 100%; border-collapse: collapse; min-width: 640px; }
.lp-compare th, .lp-compare td { padding: 14px 18px; text-align: left; border-bottom: 1px solid var(--lp-border); font-size: .92rem; }
.lp-compare thead th { background: var(--lp-bg-elev); font-weight: 650; color: var(--lp-text-soft); font-size: .85rem; text-transform: uppercase; letter-spacing: .05em; }
.lp-compare tbody tr:last-child td { border-bottom: none; }
.lp-compare tbody tr:hover { background: rgba(255,255,255,.02); }
.lp-compare__hl { background: rgba(37,99,235,.08); color: var(--lp-text); }
.lp-compare .yes { color: var(--lp-green); font-weight: 600; }
.lp-compare .no { color: var(--lp-text-dim); }
.lp-compare .meh { color: var(--lp-amber); }

/* install */
.lp-install { display: grid; grid-template-columns: repeat(3, 1fr); gap: 24px; }
.lp-install__col { display: flex; flex-direction: column; gap: 14px; }
.lp-install__title { font-size: 1.1rem; font-weight: 650; }
.lp-install__text { color: var(--lp-text-soft); font-size: .95rem; }
.lp-platforms { display: flex; flex-wrap: wrap; gap: 8px; }
.lp-platform { font-size: .8rem; padding: 5px 11px; border-radius: 999px; background: var(--lp-surface); border: 1px solid var(--lp-border); color: var(--lp-text-soft); }
.lp-quickstart { margin-top: 48px; background: var(--lp-surface); border: 1px solid var(--lp-border); border-radius: var(--lp-radius); padding: 28px 32px; }
.lp-quickstart h3 { font-size: 1.15rem; margin-bottom: 16px; }
.lp-quickstart ol { padding-left: 20px; display: flex; flex-direction: column; gap: 10px; color: var(--lp-text-soft); }
.lp-quickstart ol li { padding-left: 4px; }
.lp-quickstart ol li::marker { color: var(--lp-blue-bright); font-weight: 700; }

/* CTA */
.lp-cta { padding: 88px 0; text-align: center; background: radial-gradient(ellipse 70% 80% at 50% 0%, rgba(37,99,235,.18), transparent 60%), var(--lp-bg-soft); border-top: 1px solid var(--lp-border-soft); }
.lp-cta h2 { font-size: clamp(1.8rem, 4vw, 2.6rem); font-weight: 800; letter-spacing: -.02em; margin: 0; }
.lp-cta p { margin-top: 12px; color: var(--lp-text-soft); font-size: 1.08rem; }
.lp-cta__btns { margin-top: 28px; display: flex; gap: 14px; justify-content: center; flex-wrap: wrap; }

/* footer */
.lp-footer { background: var(--lp-bg-soft); border-top: 1px solid var(--lp-border); padding: 56px 0 28px; }
.lp-footer__inner { display: grid; grid-template-columns: 1.4fr 2fr; gap: 48px; }
.lp-footer__tag { margin-top: 12px; color: var(--lp-text-dim); font-size: .92rem; max-width: 320px; }
.lp-footer__cols { display: grid; grid-template-columns: repeat(3, 1fr); gap: 24px; }
.lp-footer__col { display: flex; flex-direction: column; gap: 10px; }
.lp-footer__col h5 { font-size: .82rem; text-transform: uppercase; letter-spacing: .08em; color: var(--lp-text-dim); margin-bottom: 4px; }
.lp-footer__col a { color: var(--lp-text-soft); font-size: .94rem; transition: color .15s var(--lp-ease); }
.lp-footer__col a:hover { color: var(--lp-text); }
.lp-footer__bottom { margin-top: 40px; padding-top: 22px; border-top: 1px solid var(--lp-border-soft); display: flex; justify-content: space-between; flex-wrap: wrap; gap: 10px; color: var(--lp-text-dim); font-size: .85rem; }

/* reveal animation */
.lp-reveal { opacity: 0; transform: translateY(18px); transition: opacity .6s var(--lp-ease), transform .6s var(--lp-ease); }
.lp-reveal.in { opacity: 1; transform: none; }

/* responsive */
@media (max-width: 980px) {
  .lp-hero__inner { grid-template-columns: 1fr; gap: 40px; }
  .lp-split { grid-template-columns: 1fr; gap: 36px; }
  .lp-ai { grid-template-columns: 1fr; gap: 36px; }
  .lp-grid--3 { grid-template-columns: repeat(2, 1fr); }
  .lp-grid--4 { grid-template-columns: repeat(2, 1fr); }
  .lp-install { grid-template-columns: 1fr; }
  .lp-footer__inner { grid-template-columns: 1fr; gap: 32px; }
  .lp-nav__links { display: none; }
  .lp-nav__toggle { display: flex; }
  .lp-section { padding: 72px 0; }
}

@media (max-width: 620px) {
  .lp-container { padding: 0 18px; }
  .lp-grid--3, .lp-grid--4 { grid-template-columns: 1fr; }
  .lp-hero__stats { gap: 18px; }
  .lp-nav__actions .lp-btn--ghost span { display: none; }
  .lp-footer__cols { grid-template-columns: repeat(2, 1fr); }
  .lp-footer__bottom { justify-content: flex-start; }
}

@media (prefers-reduced-motion: reduce) {
  .lp * { animation: none !important; transition: none !important; }
  .lp-reveal { opacity: 1; transform: none; }
}
</style>
