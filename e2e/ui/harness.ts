// Test harness: runs the real `cloud_home` binary (it serves the built ui/dist from disk in
// debug builds) in a throwaway directory, and drives it with a real Chromium via playwright-core.
import { spawn, type ChildProcess } from 'node:child_process'
import { mkdtempSync, rmSync, mkdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { chromium, type Browser } from 'playwright-core'

export const REPO = resolve(import.meta.dir, '../..')
export const ADMIN_PASSWORD = 'e2e-admin-pw-1234'
export const USER_PASSWORD = 'correct-horse-battery'
const READY = 'Cloud Home 后端已启动'

export interface Server {
  url: string
  dir: string
  stop(): void
}

/** Starts the server on a free port. The port is confirmed by the server's own startup log line,
 *  and the start is retried on another port when the bind lost a race. */
export async function startServer(env: Record<string, string> = {}): Promise<Server> {
  const dir = mkdtempSync(join(tmpdir(), 'ch-ui-e2e-'))
  mkdirSync(join(dir, 'drive'))
  for (let attempt = 0; attempt < 5; attempt++) {
    const port = await freePort()
    const child: ChildProcess = spawn(join(REPO, 'target/debug/cloud_home'), [], {
      cwd: dir,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: {
        PATH: process.env.PATH ?? '',
        SERVER_HOST: '127.0.0.1',
        SERVER_PORT: String(port),
        JWT_SECRET: 'e2e-throwaway-jwt-secret',
        ADMIN_PASSWORD,
        DEV_DRIVE_PATH: join(dir, 'drive'),
        VOLUMES_WATCH_PATH: join(dir, 'no-volumes'),
        LOG_DIR: join(dir, 'logs'),
        UPDATE_ROOT: join(dir, 'update'),
        REGISTRATION_MODE: 'open',
        CHAT_PROVIDER: 'none',
        ...env,
      },
    })
    let log = ''
    child.stdout?.on('data', (d) => (log += d))
    child.stderr?.on('data', (d) => (log += d))
    const started = await new Promise<boolean>((done) => {
      const timer = setInterval(() => {
        if (log.includes(READY)) (clearInterval(timer), done(true))
      }, 50)
      child.on('exit', () => (clearInterval(timer), done(false)))
    })
    if (started) {
      return {
        url: `http://127.0.0.1:${port}`,
        dir,
        stop() {
          child.kill()
          rmSync(dir, { recursive: true, force: true })
        },
      }
    }
  }
  rmSync(dir, { recursive: true, force: true })
  throw new Error('cloud_home did not start after 5 attempts')
}

async function freePort(): Promise<number> {
  const l = Bun.listen({ hostname: '127.0.0.1', port: 0, socket: { data() {} } })
  const port = l.port
  l.stop(true)
  return port
}

export async function launch(): Promise<Browser> {
  return chromium.launch({ headless: true })
}
