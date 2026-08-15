/**
 * Session timeout warning — shows a dialog 5 minutes before the session
 * idle timeout (30 min from the backend) expires.
 *
 * Tracks the last user interaction (mouse, keyboard, touch, scroll). When
 * 25 minutes have elapsed without activity, a warning dialog appears with
 * a "Stay signed in" button that calls the refresh endpoint to extend the
 * session. If the user does not respond within 5 more minutes, they are
 * logged out and redirected to the login page.
 */
import { useEffect, useState, useCallback, useRef } from "react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { useAuth } from "./AuthContext"
import { refreshApi } from "./api"
import { getRefreshToken } from "@/lib/api/client"

const IDLE_TIMEOUT_MS = 30 * 60 * 1000
const WARNING_BEFORE_MS = 5 * 60 * 1000
const WARNING_AT_MS = IDLE_TIMEOUT_MS - WARNING_BEFORE_MS

export function SessionTimeoutWarning() {
  const { isAuthenticated, logout } = useAuth()
  const [showWarning, setShowWarning] = useState(false)
  const lastActivityRef = useRef(Date.now())
  const warningShownRef = useRef(false)

  const resetActivity = useCallback(() => {
    lastActivityRef.current = Date.now()
    if (showWarning) {
      setShowWarning(false)
      warningShownRef.current = false
    }
  }, [showWarning])

  useEffect(() => {
    if (!isAuthenticated) return

    const events = ["mousedown", "keydown", "touchstart", "scroll", "mousemove"]
    const handler = () => {
      lastActivityRef.current = Date.now()
      if (showWarning) {
        setShowWarning(false)
        warningShownRef.current = false
      }
    }
    events.forEach((e) => window.addEventListener(e, handler, { passive: true }))

    const interval = setInterval(() => {
      const elapsed = Date.now() - lastActivityRef.current
      if (elapsed >= IDLE_TIMEOUT_MS) {
        void logout()
      } else if (elapsed >= WARNING_AT_MS && !warningShownRef.current) {
        setShowWarning(true)
        warningShownRef.current = true
      }
    }, 5000)

    return () => {
      events.forEach((e) => window.removeEventListener(e, handler))
      clearInterval(interval)
    }
  }, [isAuthenticated, logout, showWarning])

  const handleStaySignedIn = useCallback(async () => {
    const refreshToken = getRefreshToken()
    if (refreshToken) {
      try {
        await refreshApi(refreshToken)
      } catch {
        void logout()
        return
      }
    }
    resetActivity()
  }, [logout, resetActivity])

  if (!isAuthenticated) return null

  return (
    <Dialog open={showWarning} onOpenChange={(open) => {
      if (!open) resetActivity()
    }}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>Session expiring</DialogTitle>
          <DialogDescription>
            Your session will expire in 5 minutes due to inactivity.
            Click "Stay signed in" to extend your session.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => void logout()}>
            Sign out now
          </Button>
          <Button onClick={() => void handleStaySignedIn()}>
            Stay signed in
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
