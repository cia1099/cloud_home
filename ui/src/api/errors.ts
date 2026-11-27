import { ApiError } from './client'
import { en, type MessageKey } from '../i18n/messages/en'

function isMessageKey(key: string): key is MessageKey {
  return key in en
}

/** Maps an API error `code` (or a client-side `NETWORK`) to its localized message key. */
export function errorKey(error: unknown): MessageKey {
  const code = error instanceof ApiError ? error.code : 'UNKNOWN'
  const key = `error.${code}`
  return isMessageKey(key) ? key : 'error.UNKNOWN'
}
