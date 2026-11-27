import { INTL_TAGS, type Locale } from '../i18n/locale'

/**
 * Calling codes by ISO 3166-1 region. +1 is shared with the Caribbean NANP territories: choose
 * United States and type the number with its area code (the E.164 string is the same).
 */
const DIAL =
  'AD376 AE971 AF93 AL355 AM374 AO244 AR54 AT43 AU61 AW297 AZ994 BA387 BD880 BE32 BF226 BG359 BH973 BI257 BJ229 BN673 BO591 BR55 BT975 BW267 BY375 BZ501 ' +
  'CA1 CD243 CF236 CG242 CH41 CI225 CL56 CM237 CN86 CO57 CR506 CU53 CV238 CY357 CZ420 DE49 DJ253 DK45 DZ213 EC593 EE372 EG20 ER291 ES34 ET251 FI358 FJ679 ' +
  'FO298 FR33 GA241 GB44 GE995 GH233 GL299 GM220 GN224 GQ240 GR30 GT502 GW245 GY592 HK852 HN504 HR385 HT509 HU36 ID62 IE353 IL972 IN91 IQ964 IR98 IS354 ' +
  'IT39 JO962 JP81 KE254 KG996 KH855 KM269 KR82 KW965 KZ7 LA856 LB961 LI423 LK94 LR231 LS266 LT370 LU352 LV371 LY218 MA212 MC377 MD373 ME382 MG261 MK389 ' +
  'ML223 MM95 MN976 MO853 MR222 MT356 MU230 MV960 MW265 MX52 MY60 MZ258 NA264 NC687 NE227 NG234 NI505 NL31 NO47 NP977 NZ64 OM968 PA507 PE51 PF689 PG675 ' +
  'PH63 PK92 PL48 PS970 PT351 PY595 QA974 RO40 RS381 RU7 RW250 SA966 SB677 SC248 SD249 SE46 SG65 SI386 SK421 SL232 SM378 SN221 SO252 SR597 SS211 SV503 ' +
  'SY963 SZ268 TD235 TG228 TH66 TJ992 TL670 TM993 TN216 TO676 TR90 TW886 TZ255 UA380 UG256 US1 UY598 UZ998 VE58 VN84 VU678 WS685 XK383 YE967 ZA27 ZM260 ZW263'

export interface Country {
  region: string
  /** Calling code without `+`. */
  dial: string
}

export const COUNTRIES: readonly Country[] = DIAL.split(' ').map((entry) => ({ region: entry.slice(0, 2), dial: entry.slice(2) }))

/** Shared calling codes resolve to one region when an E.164 number is split. */
const PREFERRED: Record<string, string> = { '1': 'US', '7': 'RU' }

/** Languages suggest their own regions first (design/app-spec.md section 12.5). */
export const SUGGESTED: Record<Locale, readonly string[]> = {
  en: ['US', 'CA', 'GB'],
  'zh-Hans': ['CN', 'SG', 'MY'],
  'zh-Hant': ['TW', 'HK', 'MO'],
}

/** Names Intl does not give the way the glossary wants them (Taiwan wording for Traditional). */
const NAME_OVERRIDES: Partial<Record<Locale, Record<string, string>>> = {
  'zh-Hans': { CN: '中国大陆', HK: '中国香港', MO: '中国澳门', TW: '中国台湾' },
  'zh-Hant': { CN: '中國大陸', HK: '香港', MO: '澳門', TW: '臺灣', KR: '南韓' },
  en: { CN: 'Mainland China' },
}

export function byRegion(region: string): Country {
  const found = COUNTRIES.find((c) => c.region === region)
  if (!found) throw new Error(`unknown region ${region}`)
  return found
}

export function regionName(locale: Locale, region: string): string {
  const override = NAME_OVERRIDES[locale]?.[region]
  if (override) return override
  return new Intl.DisplayNames(INTL_TAGS[locale], { type: 'region' }).of(region) ?? region
}

export interface NamedCountry extends Country {
  name: string
}

/** Every country with its localized name, sorted with the language's own collation. */
export function listCountries(locale: Locale): NamedCountry[] {
  const collator = new Intl.Collator(INTL_TAGS[locale])
  return COUNTRIES.map((c) => ({ ...c, name: regionName(locale, c.region) })).sort((a, b) => collator.compare(a.name, b.name))
}

/** Matches the localized name, the ISO code, or the calling code with or without `+`. */
export function filterCountries(countries: readonly NamedCountry[], query: string): NamedCountry[] {
  const q = query.trim().toLowerCase().replace(/^\+/, '')
  if (!q) return [...countries]
  return countries.filter((c) => c.name.toLowerCase().includes(q) || c.region.toLowerCase() === q || c.dial.startsWith(q))
}

/** National digit counts per country (after removing the trunk prefix); the default is 4 to 14. */
const NATIONAL_LENGTH: Record<string, [number, number]> = {
  US: [10, 10], CA: [10, 10], GB: [10, 10], AU: [9, 9], CN: [11, 11], TW: [9, 9], HK: [8, 8], MO: [8, 8],
  JP: [10, 10], KR: [9, 10], SG: [8, 8], MY: [9, 10], DE: [7, 12], FR: [9, 9], IN: [10, 10], NZ: [8, 10],
}

/** Italian-style numbers keep their leading 0; everywhere else a single leading 0 is the trunk prefix. */
const KEEPS_LEADING_ZERO = new Set(['IT', 'SM'])

export type PhoneCheck = { ok: true; e164: string } | { ok: false }

/** Builds `+<dial><national>` from what the user typed; run on blur and Send Code, never while typing. */
export function checkPhone(country: Country, input: string): PhoneCheck {
  if (!/^[\d\s\-().]*$/.test(input)) return { ok: false }
  let digits = input.replace(/\D/g, '')
  if (!KEEPS_LEADING_ZERO.has(country.region)) digits = digits.replace(/^0/, '')
  const [min, max] = NATIONAL_LENGTH[country.region] ?? [4, 14]
  const total = country.dial.length + digits.length
  if (digits.length < min || digits.length > max || total < 8 || total > 15) return { ok: false }
  return { ok: true, e164: `+${country.dial}${digits}` }
}

/** Splits an E.164 number into its region and national digits, or `null` when no calling code fits. */
export function splitPhone(e164: string): { country: Country; national: string } | null {
  if (!/^\+\d{8,15}$/.test(e164)) return null
  const digits = e164.slice(1)
  for (const length of [1, 2, 3]) {
    const matches = COUNTRIES.filter((c) => c.dial === digits.slice(0, length))
    if (matches.length === 0) continue
    const country = matches.find((c) => c.region === PREFERRED[c.dial]) ?? matches[0]
    return { country, national: digits.slice(length) }
  }
  return null
}

/** `+886 912345678`: the calling code separated from the rest, so it reads the same in every script. */
export function formatPhone(e164: string): string {
  const parts = splitPhone(e164)
  return parts ? `+${parts.country.dial} ${parts.national}` : e164
}
