import { Routes, Route, Navigate } from 'react-router-dom'
import { isAuthenticated } from './lib/api'
import Login from './pages/Login'
import Register from './pages/Register'
import Dashboard from './pages/Dashboard'
import Licenses from './pages/Licenses'
import Billing from './pages/Billing'
import Team from './pages/Team'
import Layout from './components/Layout'

function Protected({ children }: { children: React.ReactNode }) {
  if (!isAuthenticated()) {
    return <Navigate to="/login" replace />
  }
  return <>{children}</>
}

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/register" element={<Register />} />
      <Route
        path="/"
        element={
          <Protected>
            <Layout />
          </Protected>
        }
      >
        <Route index element={<Dashboard />} />
        <Route path="licenses" element={<Licenses />} />
        <Route path="billing" element={<Billing />} />
        <Route path="team" element={<Team />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}
