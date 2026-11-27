import { Icon, type IconName } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import type { MessageKey } from '../i18n/messages/en'

/** Full-screen outcomes of sign-in and registration; none of them is a route (design/app-spec.md section 1). */
export type StatusKind = 'submitted' | 'account-pending' | 'rejected' | 'closed' | 'busy'

const SCREENS: Record<StatusKind, { mark: 'acc' | 'warn' | 'err'; icon: IconName; title: MessageKey; body: MessageKey }> = {
  submitted: { mark: 'acc', icon: 'mail', title: 'approval.submitted.title', body: 'approval.submitted.body' },
  'account-pending': { mark: 'warn', icon: 'clock', title: 'approval.pending.title', body: 'approval.pending.body' },
  rejected: { mark: 'err', icon: 'alert', title: 'approval.rejected.title', body: 'approval.rejected.body' },
  closed: { mark: 'warn', icon: 'alert', title: 'approval.closed.title', body: 'approval.closed.body' },
  busy: { mark: 'warn', icon: 'clock', title: 'approval.busy.title', body: 'approval.busy.body' },
}

export function StatusScreen({ kind, onBack }: { kind: StatusKind; onBack: () => void }) {
  const { t } = useI18n()
  const screen = SCREENS[kind]
  return (
    <>
      <span className={`statusmark ${screen.mark}`} aria-hidden="true">
        <Icon name={screen.icon} size={34} />
      </span>
      <h1>{t(screen.title)}</h1>
      <p className="sub">{t(screen.body)}</p>
      {kind === 'submitted' && (
        <ol className="steps">
          <li className="s done">
            <span className="n">
              <Icon name="check" size={16} />
            </span>
            {t('approval.step1')}
          </li>
          <li className="s now" aria-current="step">
            <span className="n">
              <Icon name="clock" size={16} />
            </span>
            {t('approval.step2')}
            <span className="chip warn">{t('approval.waiting')}</span>
          </li>
          <li className="s">
            <span className="n">3</span>
            {t('approval.step3')}
          </li>
        </ol>
      )}
      <button type="button" className="btn plain block lg" onClick={onBack}>
        {t('approval.back_to_sign_in')}
      </button>
    </>
  )
}
