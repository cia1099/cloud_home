import { useId, useState, type Ref } from 'react'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'

interface Props {
  value: string
  onChange: (value: string) => void
  invalid?: boolean
  autoComplete: 'current-password' | 'new-password'
  hint?: string
  ref?: Ref<HTMLInputElement>
}

/** Password input with a show/hide button; labelled, and `hint` is announced with it. */
export function PasswordField({ value, onChange, invalid, autoComplete, hint, ref }: Props) {
  const { t } = useI18n()
  const [visible, setVisible] = useState(false)
  const id = useId()
  return (
    <div className="field">
      <label htmlFor={id}>{t('auth.password')}</label>
      <div className="pw">
        <input
          id={id}
          ref={ref}
          className="input"
          type={visible ? 'text' : 'password'}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          autoComplete={autoComplete}
          aria-invalid={invalid || undefined}
          aria-describedby={hint ? `${id}-hint` : undefined}
          required
        />
        <button
          type="button"
          className="ibtn"
          aria-pressed={visible}
          aria-label={t(visible ? 'auth.hide_password' : 'auth.show_password')}
          onClick={() => setVisible((v) => !v)}
        >
          <Icon name={visible ? 'eyeOff' : 'eye'} />
        </button>
      </div>
      {hint && (
        <p id={`${id}-hint`} className="hint">
          {hint}
        </p>
      )}
    </div>
  )
}
