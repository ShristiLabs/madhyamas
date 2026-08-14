/**
 * UsersPanel — user management table with create/edit/delete dialogs.
 *
 * Admin-only. Uses TanStack Query for data fetching and mutations.
 * API: GET/POST /api/users, PUT/DELETE /api/users/:id.
 */
import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Loader2, Plus, Trash2, Pencil, UserCog } from "lucide-react"
import {
  listUsersApi,
  createUserApi,
  updateUserApi,
  deleteUserApi,
  type AdminUser,
  type CreateUserPayload,
  type UpdateUserPayload,
} from "@/lib/api/admin"
import { useToast } from "@/components/ui/use-toast"
import { ApiError } from "@/lib/api/client"

const ROLES = ["admin", "user", "viewer", "readonly"]
const STATUSES = ["active", "inactive", "suspended"]

export function UsersPanel() {
  const { toast } = useToast()
  const queryClient = useQueryClient()
  const { data: users, isLoading } = useQuery({
    queryKey: ["admin-users"],
    queryFn: listUsersApi,
  })
  const [createOpen, setCreateOpen] = useState(false)
  const [editUser, setEditUser] = useState<AdminUser | null>(null)
  const [deleteUser, setDeleteUser] = useState<AdminUser | null>(null)

  const createMut = useMutation({
    mutationFn: (data: CreateUserPayload) => createUserApi(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-users"] })
      setCreateOpen(false)
      toast({ title: "User created" })
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to create user",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  const updateMut = useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateUserPayload }) =>
      updateUserApi(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-users"] })
      setEditUser(null)
      toast({ title: "User updated" })
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to update user",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  const deleteMut = useMutation({
    mutationFn: (id: string) => deleteUserApi(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-users"] })
      setDeleteUser(null)
      toast({ title: "User deleted" })
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to delete user",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading users…
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-border px-4 py-2">
        <h2 className="text-sm font-semibold">User Management</h2>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="mr-1 h-3.5 w-3.5" /> Add User
        </Button>
      </div>
      <div className="flex-1 overflow-auto">
        <table className="w-full text-xs">
          <thead className="sticky top-0 bg-card text-left text-muted-foreground">
            <tr className="border-b border-border">
              <th className="px-4 py-2 font-medium">Username</th>
              <th className="px-4 py-2 font-medium">Email</th>
              <th className="px-4 py-2 font-medium">Role</th>
              <th className="px-4 py-2 font-medium">Status</th>
              <th className="px-4 py-2 font-medium">Created</th>
              <th className="px-4 py-2 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {users?.map((u) => (
              <tr key={u.id} className="border-b border-border/50 hover:bg-muted/30">
                <td className="px-4 py-2 font-medium">{u.username}</td>
                <td className="px-4 py-2 text-muted-foreground">{u.email || "—"}</td>
                <td className="px-4 py-2">
                  <span className="rounded bg-primary/10 px-1.5 py-0.5 text-2xs font-medium text-primary">
                    {u.role}
                  </span>
                </td>
                <td className="px-4 py-2">
                  <span className={
                    u.status === "active"
                      ? "rounded bg-success/10 px-1.5 py-0.5 text-2xs font-medium text-success"
                      : "rounded bg-warning/10 px-1.5 py-0.5 text-2xs font-medium text-warning"
                  }>
                    {u.status}
                  </span>
                </td>
                <td className="px-4 py-2 text-muted-foreground">
                  {u.created_at ? new Date(u.created_at * 1000).toLocaleDateString() : "—"}
                </td>
                <td className="px-4 py-2">
                  <div className="flex gap-1">
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      onClick={() => setEditUser(u)}
                      title="Edit user"
                    >
                      <Pencil className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      onClick={() => setDeleteUser(u)}
                      title="Delete user"
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </td>
              </tr>
            ))}
            {users?.length === 0 && (
              <tr>
                <td colSpan={6} className="px-4 py-8 text-center text-muted-foreground">
                  No users found.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <CreateUserDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        onSubmit={(d) => createMut.mutate(d)}
        loading={createMut.isPending}
      />
      <EditUserDialog
        user={editUser}
        onOpenChange={(open) => !open && setEditUser(null)}
        onSubmit={(d) => editUser && updateMut.mutate({ id: editUser.id, data: d })}
        loading={updateMut.isPending}
      />
      <DeleteConfirmDialog
        user={deleteUser}
        onOpenChange={(open) => !open && setDeleteUser(null)}
        onConfirm={() => deleteUser && deleteMut.mutate(deleteUser.id)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}

function CreateUserDialog({ open, onOpenChange, onSubmit, loading }: {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: CreateUserPayload) => void
  loading: boolean
}) {
  const [username, setUsername] = useState("")
  const [email, setEmail] = useState("")
  const [password, setPassword] = useState("")
  const [role, setRole] = useState("user")

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    onSubmit({ username, email, password, role })
    setUsername("")
    setEmail("")
    setPassword("")
    setRole("user")
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>Create User</DialogTitle>
          <DialogDescription>Add a new user account.</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="cu-username">Username</Label>
            <Input id="cu-username" value={username} onChange={(e) => setUsername(e.target.value)} required />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="cu-email">Email</Label>
            <Input id="cu-email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="cu-password">Password</Label>
            <Input id="cu-password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} required />
          </div>
          <div className="space-y-1.5">
            <Label>Role</Label>
            <Select value={role} onValueChange={setRole}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ROLES.map((r) => (
                  <SelectItem key={r} value={r}>{r}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <DialogFooter>
            <Button type="submit" disabled={loading}>
              {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Create
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function EditUserDialog({ user, onOpenChange, onSubmit, loading }: {
  user: AdminUser | null
  onOpenChange: (open: boolean) => void
  onSubmit: (data: UpdateUserPayload) => void
  loading: boolean
}) {
  const [role, setRole] = useState(user?.role ?? "user")
  const [status, setStatus] = useState(user?.status ?? "active")
  const [password, setPassword] = useState("")

  // Reset when user changes
  const userKey = user?.id
  const [lastKey, setLastKey] = useState<string | null>(null)
  if (userKey && userKey !== lastKey) {
    setRole(user!.role)
    setStatus(user!.status)
    setPassword("")
    setLastKey(userKey)
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const data: UpdateUserPayload = { role, status }
    if (password) data.password = password
    onSubmit(data)
    setPassword("")
  }

  return (
    <Dialog open={!!user} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <UserCog className="h-4 w-4" /> Edit {user?.username}
          </DialogTitle>
          <DialogDescription>Update role, status, or reset password.</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-3">
          <div className="space-y-1.5">
            <Label>Role</Label>
            <Select value={role} onValueChange={setRole}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ROLES.map((r) => (
                  <SelectItem key={r} value={r}>{r}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-1.5">
            <Label>Status</Label>
            <Select value={status} onValueChange={setStatus}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {STATUSES.map((s) => (
                  <SelectItem key={s} value={s}>{s}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="eu-password">Reset Password (leave blank to keep)</Label>
            <Input id="eu-password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="New password" />
          </div>
          <DialogFooter>
            <Button type="submit" disabled={loading}>
              {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Save Changes
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function DeleteConfirmDialog({ user, onOpenChange, onConfirm, loading }: {
  user: AdminUser | null
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
  loading: boolean
}) {
  return (
    <Dialog open={!!user} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>Delete User</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete <strong>{user?.username}</strong>?
            This action cannot be undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="destructive" onClick={onConfirm} disabled={loading}>
            {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
