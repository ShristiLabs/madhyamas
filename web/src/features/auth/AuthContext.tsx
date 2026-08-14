/**
 * Auth context — manages JWT token, current user, login/logout/refresh.
 *
 * The token is stored in localStorage (via the API client helpers). On
 * mount, if a token exists, `GET /api/auth/me` is called to fetch the
 * current user. If that returns 401 (and refresh also fails), the token
 * is cleared and the user is treated as unauthenticated.
 */
import { createContext, useContext, useEffect, useState, useCallback, type ReactNode } from "react"
import {
  getAuthToken,
  getRefreshToken,
  setAuthTokens,
  clearAuthTokens,
  setUnauthorizedHandler,
} from "@/lib/api/client"
import { loginApi, logoutApi, getMeApi, type AuthUser } from "./api"

interface AuthContextValue {
  user: AuthUser | null
  isAuthenticated: boolean
  isLoading: boolean
  login: (username: string, password: string) => Promise<void>
  logout: () => Promise<void>
  refreshUser: () => Promise<void>
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  const clearSession = useCallback(() => {
    clearAuthTokens()
    setUser(null)
  }, [])

  // Register the 401 handler so the API client can notify us when a
  // token is rejected and refresh fails.
  useEffect(() => {
    setUnauthorizedHandler(() => {
      setUser(null)
    })
    return () => setUnauthorizedHandler(null)
  }, [])

  // On mount: if a token exists, validate it by fetching the current user.
  useEffect(() => {
    const token = getAuthToken()
    if (!token) {
      setIsLoading(false)
      return
    }
    getMeApi()
      .then((u) => {
        setUser(u)
        setIsLoading(false)
      })
      .catch(() => {
        // 401 handler in the API client already cleared tokens + retried.
        // If we get here, the session is invalid.
        clearSession()
        setIsLoading(false)
      })
  }, [clearSession])

  const login = useCallback(async (username: string, password: string) => {
    const resp = await loginApi(username, password)
    setAuthTokens(resp.token, resp.refresh_token)
    setUser(resp.user)
  }, [])

  const logout = useCallback(async () => {
    try {
      await logoutApi()
    } catch {
      // ignore — we clear locally regardless
    } finally {
      clearSession()
    }
  }, [clearSession])

  const refreshUser = useCallback(async () => {
    const u = await getMeApi()
    setUser(u)
  }, [])

  return (
    <AuthContext.Provider
      value={{
        user,
        isAuthenticated: user !== null,
        isLoading,
        login,
        logout,
        refreshUser,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}

// eslint-disable-next-line react-refresh/only-export-components
export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error("useAuth must be used within AuthProvider")
  return ctx
}

/** Check if a refresh token exists in storage (without calling the API). */
// eslint-disable-next-line react-refresh/only-export-components
export function hasRefreshToken(): boolean {
  return getRefreshToken() !== null
}
