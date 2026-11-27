import { useNavigate } from 'react-router'
import { AuthLayout } from './AuthLayout'
import { StatusScreen } from './StatusScreen'

/** "Request submitted" after `202 pending_approval`. */
export function PendingPage() {
  const navigate = useNavigate()
  return (
    <AuthLayout>
      <StatusScreen kind="submitted" onBack={() => navigate('/signin')} />
    </AuthLayout>
  )
}
