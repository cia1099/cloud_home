import { NavLink, Outlet } from 'react-router'
import { Icon, type IconName } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'
import { useStreamStatus } from '../sse/useEventStream'
import { UploadPanel } from '../upload/UploadPanel'
import { UploadProvider } from '../upload/UploadProvider'
import { useOnline } from './useOnline'

// New destinations (Shared, Trash, Storage, Administration) are added with their screens.
const NAV: { to: string; icon: IconName; label: MessageKey }[] = [
  { to: '/files', icon: 'folder', label: 'nav.files' },
  { to: '/settings', icon: 'sliders', label: 'nav.settings' },
]

function NavItems({ className }: { className: string }) {
  const { t } = useI18n()
  return NAV.map((item) => (
    <NavLink key={item.to} to={item.to} className={className}>
      <Icon name={item.icon} size={22} />
      <span>{t(item.label)}</span>
    </NavLink>
  ))
}

/** Sidebar (desktop), icon rail (tablet) or tab bar (phone) around the routed content. */
export function AppShell() {
  const { t } = useI18n()
  const online = useOnline()
  const stream = useStreamStatus()
  return (
    <UploadProvider>
    <div className="app">
      <a className="skip" href="#content">
        {t('a11y.skip')}
      </a>
      <nav className="side" aria-label={t('nav.main')}>
        <div className="brand">
          <span className="logo" aria-hidden="true">
            <Icon name="cloud" size={18} />
          </span>
          <span>{t('app.name')}</span>
        </div>
        <NavItems className="navi" />
      </nav>
      <div className="main">
        {!online && (
          <div className="strip offline" role="status">
            <Icon name="wifioff" size={16} />
            {t('offline.strip')}
          </div>
        )}
        {online && stream === 'reconnecting' && (
          <div className="strip live" role="status">
            <Icon name="spinner" size={14} className="spin" />
            {t('sse.reconnecting')}
          </div>
        )}
        <div id="content" className="content" tabIndex={-1}>
          <Outlet />
        </div>
      </div>
      <nav className="tabs" aria-label={t('nav.main')}>
        <NavItems className="" />
      </nav>
      <UploadPanel />
    </div>
    </UploadProvider>
  )
}
