import { BrowserRouter, Navigate, Route, Routes } from 'react-router'
import { OAuthCallbackPage } from './auth/OAuthCallbackPage'
import { PendingPage } from './auth/PendingPage'
import { RegisterPage } from './auth/RegisterPage'
import { SignInPage } from './auth/SignInPage'
import { I18nProvider } from './i18n/I18nProvider'
import { AdminPage } from './pages/AdminPage'
import { FilesPage } from './pages/FilesPage'
import { PublicSharePage } from './pages/PublicSharePage'
import { SettingsPage } from './pages/SettingsPage'
import { SharedPage } from './pages/SharedPage'
import { StoragePage } from './pages/StoragePage'
import { TrashPage } from './pages/TrashPage'
import { SessionProvider } from './session/SessionProvider'
import { AppShell } from './shell/AppShell'
import { RequireAuth } from './shell/RequireAuth'
import { ThemeProvider } from './theme/ThemeProvider'
import { ToastProvider } from './toast/ToastProvider'

export function App() {
  return (
    <ThemeProvider>
      <I18nProvider>
        <BrowserRouter>
          <SessionProvider>
            <ToastProvider>
            <Routes>
              <Route path="/signin" element={<SignInPage />} />
              <Route path="/register" element={<RegisterPage />} />
              <Route path="/auth/callback/:provider" element={<OAuthCallbackPage />} />
              <Route path="/pending" element={<PendingPage />} />
              <Route path="/s/:token" element={<PublicSharePage />} />
              <Route element={<RequireAuth />}>
                <Route element={<AppShell />}>
                  <Route path="/files/:folderId?" element={<FilesPage />} />
                  <Route path="/shared" element={<SharedPage />} />
                  <Route path="/trash" element={<TrashPage />} />
                  <Route path="/storage" element={<StoragePage />} />
                  <Route path="/settings" element={<SettingsPage />} />
                  <Route path="/admin" element={<AdminPage />} />
                </Route>
              </Route>
              <Route path="*" element={<Navigate to="/files" replace />} />
            </Routes>
            </ToastProvider>
          </SessionProvider>
        </BrowserRouter>
      </I18nProvider>
    </ThemeProvider>
  )
}
