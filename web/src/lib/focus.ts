import type { FocusHost } from "@/types/traffic"

/**
 * Check whether a host matches a focus pattern.
 *
 * Matching is case-insensitive and supports:
 * - Exact hostname: `example.com`
 * - Suffix matching: `example.com` matches `api.example.com`
 * - Wildcard subdomain: `*.example.com` matches `api.example.com` (not the bare domain)
 * - Glob: `*api*` matches any host containing `api`
 */
export function hostMatchesPattern(host: string, pattern: string): boolean {
  const target = host.trim().replace(/\.+$/, "").toLowerCase()
  const pat = pattern.trim().replace(/\.+$/, "").toLowerCase()
  if (!target || !pat) return false

  if (pat.startsWith("*.")) {
    const suffix = pat.slice(2)
    return target.endsWith(`.${suffix}`)
  }

  if (pat.includes("*")) {
    return globMatch(target, pat)
  }

  return target === pat || target.endsWith(`.${pat}`)
}

function globMatch(text: string, pattern: string): boolean {
  const textBytes = [...text]
  const patBytes = [...pattern]
  let ti = 0
  let pi = 0
  let starPi: number | null = null
  let starTi = 0

  while (ti < textBytes.length) {
    if (pi < patBytes.length && (patBytes[pi] === "?" || patBytes[pi] === textBytes[ti])) {
      ti++
      pi++
    } else if (pi < patBytes.length && patBytes[pi] === "*") {
      starPi = pi
      starTi = ti
      pi++
    } else if (starPi !== null) {
      pi = starPi + 1
      starTi++
      ti = starTi
    } else {
      return false
    }
  }

  while (pi < patBytes.length && patBytes[pi] === "*") {
    pi++
  }

  return pi === patBytes.length
}

/**
 * Check whether a host matches any of the given focus host patterns.
 */
export function hostMatchesAnyPattern(host: string, focusHosts: FocusHost[]): boolean {
  if (focusHosts.length === 0) return false
  return focusHosts.some((fh) => hostMatchesPattern(host, fh.pattern))
}
