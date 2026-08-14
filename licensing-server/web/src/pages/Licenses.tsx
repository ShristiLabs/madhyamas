import { useEffect, useState } from 'react'
import { api, type License } from '../lib/api'

export default function Licenses() {
  const [licenses, setLicenses] = useState<License[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [selected, setSelected] = useState<License | null>(null)
  const [seats, setSeats] = useState<unknown[]>([])

  useEffect(() => {
    api
      .licenses()
      .then(setLicenses)
      .catch((err) => setError(err instanceof Error ? err.message : 'Failed to load'))
      .finally(() => setLoading(false))
  }, [])

  const viewDetail = async (lic: License) => {
    setSelected(lic)
    try {
      const s = await api.seats(lic.license_id)
      setSeats(s)
    } catch {
      setSeats([])
    }
  }

  if (loading) return <div className="text-gray-500">Loading...</div>
  if (error) return <div className="text-red-600">{error}</div>

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6 text-gray-900">Licenses</h1>
      {licenses.length === 0 ? (
        <p className="text-gray-500">No licenses found.</p>
      ) : (
        <div className="space-y-4">
          {licenses.map((lic) => (
            <div key={lic.license_id} className="bg-white p-4 rounded-lg shadow-sm">
              <div className="flex justify-between items-start">
                <div>
                  <p className="font-mono text-sm text-gray-900">{lic.license_id}</p>
                  <p className="text-sm text-gray-600">Plan: {lic.plan} | Seats: {lic.seats}</p>
                  <p className="text-sm text-gray-600">
                    Status: <span className="font-medium">{lic.status}</span>
                  </p>
                  <p className="text-sm text-gray-600">
                    Expires: {new Date(lic.expires_at).toLocaleDateString()}
                  </p>
                  <div className="mt-2 flex gap-1 flex-wrap">
                    {lic.features.map((f) => (
                      <span key={f} className="px-2 py-0.5 bg-gray-100 rounded text-xs text-gray-700">
                        {f}
                      </span>
                    ))}
                  </div>
                </div>
                <button
                  onClick={() => viewDetail(lic)}
                  className="text-sm text-blue-600 hover:underline cursor-pointer"
                >
                  View seats
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
      {selected && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center p-4">
          <div className="bg-white p-6 rounded-lg max-w-lg w-full">
            <h2 className="text-lg font-bold mb-4">Seats for {selected.license_id}</h2>
            {seats.length === 0 ? (
              <p className="text-sm text-gray-500">No seats registered.</p>
            ) : (
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b">
                    <th className="text-left py-1">Instance ID</th>
                    <th className="text-left py-1">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {seats.map((s: any, i) => (
                    <tr key={i} className="border-b">
                      <td className="py-1 font-mono text-xs">{s.instance_id}</td>
                      <td className="py-1">{s.status}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <button
              onClick={() => setSelected(null)}
              className="mt-4 text-sm text-gray-600 hover:underline cursor-pointer"
            >
              Close
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
