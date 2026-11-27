import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

// The product has one version (R-026): the UI bakes in the Cargo.toml version.
function cargoVersion(): string {
  const toml = readFileSync(fileURLToPath(new URL('../Cargo.toml', import.meta.url)), 'utf8')
  const match = /^version\s*=\s*"([^"]+)"/m.exec(toml)
  if (!match) throw new Error('version not found in Cargo.toml')
  return match[1]
}

const version = cargoVersion()

export default defineConfig({
  plugins: [
    react(),
    {
      // ui/dist/version.json lets the Rust release build verify the UI matches the binary.
      name: 'emit-version',
      generateBundle() {
        this.emitFile({ type: 'asset', fileName: 'version.json', source: JSON.stringify({ version }) })
      },
    },
  ],
  define: { __APP_VERSION__: JSON.stringify(version) },
  server: {
    // One origin in development: cookies, SSE and CORS behave exactly like production.
    proxy: { '/api': { target: process.env.CLOUD_HOME_API ?? 'http://127.0.0.1:8080' } },
  },
  test: { environment: 'node' },
})
