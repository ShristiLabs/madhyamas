/* ============================================================
   Madhyamas — static site interactivity
   No dependencies. Vanilla JS.
   ============================================================ */
(function () {
  "use strict";

  // ---- mobile nav toggle ----
  var toggle = document.getElementById("navToggle");
  var mobile = document.getElementById("navMobile");
  if (toggle && mobile) {
    toggle.addEventListener("click", function () {
      var open = toggle.getAttribute("aria-expanded") === "true";
      toggle.setAttribute("aria-expanded", String(!open));
      mobile.hidden = open;
    });
    // close on link click
    mobile.querySelectorAll("a").forEach(function (a) {
      a.addEventListener("click", function () {
        toggle.setAttribute("aria-expanded", "false");
        mobile.hidden = true;
      });
    });
  }

  // ---- sticky nav shadow on scroll ----
  var nav = document.getElementById("nav");
  function onScroll() {
    if (!nav) return;
    if (window.scrollY > 8) nav.classList.add("scrolled");
    else nav.classList.remove("scrolled");
  }
  onScroll();
  window.addEventListener("scroll", onScroll, { passive: true });

  // ---- footer year ----
  var year = document.getElementById("year");
  if (year) year.textContent = new Date().getFullYear();

  // ---- reveal on scroll ----
  var revealEls = document.querySelectorAll(
    ".card, .mini, .featurelist li, .callout, .codecard, .terminal, .chat, .compare tbody tr, .install__col, .quickstart"
  );
  revealEls.forEach(function (el) { el.classList.add("reveal"); });

  if ("IntersectionObserver" in window) {
    var io = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add("in");
            io.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
    );
    revealEls.forEach(function (el) { io.observe(el); });
  } else {
    revealEls.forEach(function (el) { el.classList.add("in"); });
  }
})();
