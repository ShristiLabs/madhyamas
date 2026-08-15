import { useEffect, useState } from 'react'
import { api, type MeResponse, type License } from '../lib/api'

export default function Dashboard() {
  const [me, setMe] = useState<MeResponse | null>(null)
  const [licenses, setLicenses] = useState<License[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    async function load() {
      try {
        const [meResp, licResp] = await Promise.all([api.me(), api.licenses()])
        setMe(meResp)
        setLicenses(licResp)
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load data')
      } finally {
        setLoading(false)
      }
    }
    load()
  }, [])

  if (loading) return <div className="text-gray-500">Loading...</div>
  if (error) return <div className="text-red-600">{error}</div>

  const activeLicenses = licenses.filter((l) => l.status === 'active')
  const totalSeats = activeLicenses.reduce((sum, l) => sum + l.seats, 0)

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6 text-gray-900">Dashboard</h1>
      {me && (
        <div className="bg-white p-6 rounded-lg shadow-sm mb-6">
          <h2 className="text-lg font-semibold mb-2">Account</h2>
          <p className="text-sm text-gray-600">Company: {me.name}</p>
          <p className="text-sm text-gray-600">Email: {me.email}</p>
          <p className="text-sm text-gray-600">Status: {me.status}</p>
        </div>
      )}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="bg-white p-6 rounded-lg shadow-sm">
          <p className="text-sm text-gray-500">Active Licenses</p>
          <p className="text-3xl font-bold text-gray-900">{activeLicenses.length}</p>
        </div>
        <div className="bg-white p-6 rounded-lg shadow-sm">
          <p className="text-sm text-gray-500">Total Seats</p>
          <p className="text-3xl font-bold text-gray-900">{totalSeats}</p>
        </div>
        <div className="bg-white p-6 rounded-lg shadow-sm">
          <p className="text-sm text-gray-500">Total Licenses</p>
          <p className="text-3xl font-bold text-gray-900">{licenses.length}</p>
        </div>
      </div>
      <div className="bg-white p-6 rounded-lg shadow-sm">
        <h2 className="text-lg font-semibold mb-4">Recent Licenses</h2>
        {licenses.length === 0 ? (
          <p className="text-sm text-gray-500">No licenses yet.</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200">
                <th className="text-left py-2">License ID</th>
                <th className="text-left py-2">Plan</th>
                <th className="text-left py-2">Seats</th>
                <th className="text-left py-2">Status</th>
                <th className="text-left py-2">Expires</th>
              </tr>
            </thead>
            <tbody>
              {licenses.map((lic) => (
                <tr key={lic.license_id} className="border-b border-gray-100">
                  <td className="py-2 font-mono text-xs">{lic.license_id}</td>
                  <td className="py-2">{lic.plan}</td>
                  <td className="py-2">{lic.seats}</td>
                  <td className="py-2">
                    <span
                      className={`px-2 py-1 rounded text-xs ${
                        lic.status === 'active'
                          ? 'bg-green-100 text-green-800'
                          : lic.status === 'suspended'
                            ? 'bg-yellow-100 text-yellow-800'
                            : 'bg-red-100 text-red-800'
                      }`}
                    >
                      {lic.status}
                    </span>
                  </td>
                  <td className="py-2 text-gray-600">
                    {new Date(lic.expires_at).toLocaleDateString()}
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
