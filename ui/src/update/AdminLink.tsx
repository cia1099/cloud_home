import { Link } from 'react-router'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { useAdmin } from './useAdmin'

/** Settings cell that leads to Administration, only for callers the box lets in (the phone has no sidebar). */
export function AdminLink() {
  const { t } = useI18n()
  const { access } = useAdmin()
  if (access.state !== 'locked' && access.state !== 'unlocked') return null
  return (
    <Link className="cell" to="/admin">
      <span className="lbl">{t('nav.admin')}</span>
      <Icon name="chevR" size={16} />
    </Link>
  )
}
