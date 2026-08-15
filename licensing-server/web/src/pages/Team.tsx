import { useEffect, useState } from 'react'
import { api, type TeamMember } from '../lib/api'

export default function Team() {
  const [members, setMembers] = useState<TeamMember[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [inviteEmail, setInviteEmail] = useState('')
  const [inviteRole, setInviteRole] = useState('developer')

  const load = () => {
    api
      .team()
      .then(setMembers)
      .catch((err) => setError(err instanceof Error ? err.message : 'Failed to load'))
      .finally(() => setLoading(false))
  }

  useEffect(() => {
    load()
  }, [])

  const handleInvite = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    try {
      await api.invite(inviteEmail, inviteRole)
      setInviteEmail('')
      load()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Invite failed')
    }
  }

  const handleRemove = async (id: string) => {
    try {
      await api.removeMember(id)
      load()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Remove failed')
    }
  }

  if (loading) return <div className="text-gray-500">Loading...</div>

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6 text-gray-900">Team</h1>
      {error && <div className="mb-4 text-red-600 text-sm">{error}</div>}
      <div className="bg-white p-6 rounded-lg shadow-sm mb-6">
        <h2 className="text-lg font-semibold mb-4">Invite Member</h2>
        <form onSubmit={handleInvite} className="flex gap-2">
          <input
            type="email"
            placeholder="email@example.com"
            value={inviteEmail}
            onChange={(e) => setInviteEmail(e.target.value)}
            className="flex-1 px-3 py-2 border border-gray-300 rounded-md"
            required
          />
          <select
            value={inviteRole}
            onChange={(e) => setInviteRole(e.target.value)}
            className="px-3 py-2 border border-gray-300 rounded-md"
          >
            <option value="developer">Developer</option>
            <option value="billing_admin">Billing Admin</option>
            <option value="org_admin">Org Admin</option>
          </select>
          <button
            type="submit"
            className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 cursor-pointer"
          >
            Invite
          </button>
        </form>
      </div>
      <div className="bg-white p-6 rounded-lg shadow-sm">
        <h2 className="text-lg font-semibold mb-4">Members</h2>
        {members.length === 0 ? (
          <p className="text-sm text-gray-500">No team members yet.</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200">
                <th className="text-left py-2">Email</th>
                <th className="text-left py-2">Role</th>
                <th className="text-left py-2">Status</th>
                <th className="text-left py-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              {members.map((m) => (
                <tr key={m.id} className="border-b border-gray-100">
                  <td className="py-2">{m.email}</td>
                  <td className="py-2">{m.role}</td>
                  <td className="py-2">
                    <span
                      className={`px-2 py-1 rounded text-xs ${
                        m.status === 'active'
                          ? 'bg-green-100 text-green-800'
                          : 'bg-yellow-100 text-yellow-800'
                      }`}
                    >
                      {m.status}
                    </span>
                  </td>
                  <td className="py-2">
                    <button
                      onClick={() => handleRemove(m.id)}
                      className="text-red-600 hover:underline text-xs cursor-pointer"
                    >
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
