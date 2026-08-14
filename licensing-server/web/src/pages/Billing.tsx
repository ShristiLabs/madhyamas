import { useEffect, useState } from 'react'
import { api, type BillingSummary } from '../lib/api'

export default function Billing() {
  const [billing, setBilling] = useState<BillingSummary | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    api
      .billing()
      .then(setBilling)
      .catch((err) => setError(err instanceof Error ? err.message : 'Failed to load'))
      .finally(() => setLoading(false))
  }, [])

  if (loading) return <div className="text-gray-500">Loading...</div>
  if (error) return <div className="text-red-600">{error}</div>

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6 text-gray-900">Billing</h1>
      <div className="bg-white p-6 rounded-lg shadow-sm mb-6">
        <p className="text-sm text-gray-600">
          Stripe: {billing?.stripe_configured ? 'Configured' : 'Not configured'}
        </p>
      </div>
      <div className="bg-white p-6 rounded-lg shadow-sm">
        <h2 className="text-lg font-semibold mb-4">Invoices</h2>
        {billing && billing.invoices.length > 0 ? (
          <p className="text-sm text-gray-500">Invoice list available when Stripe is configured.</p>
        ) : (
          <p className="text-sm text-gray-500">No invoices available.</p>
        )}
      </div>
    </div>
  )
}
