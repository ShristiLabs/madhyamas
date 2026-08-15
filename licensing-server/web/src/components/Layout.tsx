import { Outlet, Link, useNavigate } from 'react-router-dom'
import { clearToken } from '../lib/api'

export default function Layout() {
  const navigate = useNavigate()

  const logout = () => {
    clearToken()
    navigate('/login')
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <nav className="bg-white border-b border-gray-200 px-6 py-3 flex items-center justify-between">
        <div className="flex items-center gap-6">
          <Link to="/" className="text-lg font-bold text-gray-900">
            Madhyamas Portal
          </Link>
          <Link to="/" className="text-sm text-gray-600 hover:text-gray-900">
            Dashboard
          </Link>
          <Link to="/licenses" className="text-sm text-gray-600 hover:text-gray-900">
            Licenses
          </Link>
          <Link to="/billing" className="text-sm text-gray-600 hover:text-gray-900">
            Billing
          </Link>
          <Link to="/team" className="text-sm text-gray-600 hover:text-gray-900">
            Team
          </Link>
        </div>
        <button
          onClick={logout}
          className="text-sm text-gray-600 hover:text-gray-900 cursor-pointer"
        >
          Logout
        </button>
      </nav>
      <main className="max-w-5xl mx-auto px-6 py-8">
        <Outlet />
      </main>
    </div>
  )
}
