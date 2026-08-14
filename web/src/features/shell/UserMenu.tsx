/**
 * UserMenu — dropdown shown in the AppHeader for authenticated users.
 *
 * Displays the username, role, and a sign-out button. Only rendered in
 * the enterprise tier when the user is authenticated.
 */
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { User, LogOut } from "lucide-react"
import { useAuth } from "@/features/auth/AuthContext"

export function UserMenu() {
  const { user, logout } = useAuth()

  if (!user) return null

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="gap-1.5">
          <User className="h-3.5 w-3.5" />
          <span className="hidden max-w-[100px] truncate sm:inline">
            {user.username}
          </span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel className="flex flex-col gap-1">
          <span className="font-medium">{user.username}</span>
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="text-2xs">
              {user.role}
            </Badge>
            {user.email && (
              <span className="truncate text-2xs text-muted-foreground">
                {user.email}
              </span>
            )}
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => void logout()}>
          <LogOut className="mr-2 h-3.5 w-3.5" />
          Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
