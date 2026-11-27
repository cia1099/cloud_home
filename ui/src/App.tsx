import { BrowserRouter, Navigate, Route, Routes } from 'react-router'
import { PendingPage } from './auth/PendingPage'
import { RegisterPage } from './auth/RegisterPage'
import { SignInPage } from './auth/SignInPage'
import { I18nProvider } from './i18n/I18nProvider'
import { FilesPage } from './pages/FilesPage'
import { SettingsPage } from './pages/SettingsPage'
import { SessionProvider } from './session/SessionProvider'
import { AppShell } from './shell/AppShell'
import { RequireAuth } from './shell/RequireAuth'
import { ThemeProvider } from './theme/ThemeProvider'

export function App() {
  return (
    <ThemeProvider>
      <I18nProvider>
        <BrowserRouter>
          <SessionProvider>
            <Routes>
              <Route path="/signin" element={<SignInPage />} />
              <Route path="/register" element={<RegisterPage />} />
              <Route path="/pending" element={<PendingPage />} />
              <Route element={<RequireAuth />}>
                <Route element={<AppShell />}>
                  <Route path="/files/:folderId?" element={<FilesPage />} />
                  <Route path="/settings" element={<SettingsPage />} />
                </Route>
              </Route>
              <Route path="*" element={<Navigate to="/files" replace />} />
            </Routes>
          </SessionProvider>
        </BrowserRouter>
      </I18nProvider>
    </ThemeProvider>
  )
}
