import { useId, useMemo, useRef, useState } from 'react'
import { Icon } from '../components/Icon'
import { useI18n } from '../i18n/useI18n'
import { byRegion, filterCountries, listCountries, regionName, SUGGESTED, type Country, type NamedCountry } from './countries'

interface Props {
  country: Country
  onCountry: (country: Country) => void
  value: string
  onChange: (value: string) => void
  onBlur: () => void
  invalid: boolean
  /** Id of the element that describes the field (the error line). */
  describedBy?: string
}

/** `[CC +n]` button plus the national number; the country list opens inline, so it also works inside a dialog. */
export function PhoneField({ country, onCountry, value, onChange, onBlur, invalid, describedBy }: Props) {
  const { t, locale } = useI18n()
  const id = useId()
  const button = useRef<HTMLButtonElement>(null)
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const all = useMemo(() => listCountries(locale), [locale])
  const matches = filterCountries(all, query)
  const suggested = query.trim() ? [] : SUGGESTED[locale].map((region) => ({ ...byRegion(region), name: regionName(locale, region) }))

  const close = () => {
    setOpen(false)
    setQuery('')
    button.current?.focus()
  }
  const choose = (next: NamedCountry) => {
    onCountry({ region: next.region, dial: next.dial })
    close()
  }

  const row = (c: NamedCountry, prefix: string) => (
    <button
      key={prefix + c.region}
      type="button"
      role="option"
      aria-selected={c.region === country.region}
      className={`picker-row${c.region === country.region ? ' on' : ''}`}
      onClick={() => choose(c)}
    >
      <span className="grow">{c.name}</span>
      <span className="dim">+{c.dial}</span>
      {c.region === country.region && <Icon name="check" size={18} />}
    </button>
  )

  return (
    <div className="field">
      <label htmlFor={`${id}-tel`}>{t('auth.phone')}</label>
      <div className="phone-row">
        <button
          ref={button}
          type="button"
          className="cc-btn"
          aria-haspopup="listbox"
          aria-expanded={open}
          aria-label={t('account.country.label', { name: regionName(locale, country.region), dial: country.dial })}
          onClick={() => setOpen((v) => !v)}
        >
          {country.region} +{country.dial}
          <Icon name="chevD" size={14} />
        </button>
        <input
          id={`${id}-tel`}
          className="input"
          type="tel"
          inputMode="tel"
          autoComplete="tel-national"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onBlur={onBlur}
          aria-invalid={invalid || undefined}
          aria-describedby={describedBy}
        />
      </div>
      {open && (
        <div
          className="cc-panel"
          onKeyDown={(event) => {
            if (event.key === 'Escape') {
              event.stopPropagation()
              close()
            }
          }}
        >
          <input
            className="input"
            type="search"
            // The panel was just opened by the user: moving focus to its search field is the expected behaviour.
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t('account.country.search')}
            aria-label={t('account.country.search')}
            autoComplete="off"
            spellCheck={false}
          />
          <div className="picker" role="listbox" aria-label={t('account.country.all')}>
            {suggested.length > 0 && (
              <>
                <h3 className="picker-h">{t('account.country.suggested')}</h3>
                {suggested.map((c) => row(c, 's-'))}
                <h3 className="picker-h">{t('account.country.all')}</h3>
              </>
            )}
            {matches.map((c) => row(c, 'a-'))}
            {matches.length === 0 && <p className="hint pad-s">{t('account.country.none')}</p>}
          </div>
        </div>
      )}
    </div>
  )
}
