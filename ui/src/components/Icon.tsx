// Line icons from design/mockups/app/assets/icons.js: 24 px grid, 1.75 px stroke, currentColor.
const PATHS = {
  cloud: 'M7.5 18.5a4.5 4.5 0 0 1-.6-8.96A5.6 5.6 0 0 1 17.7 9a4.8 4.8 0 0 1-.7 9.5z',
  folder: 'M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2.2h7.5A2.5 2.5 0 0 1 21 9.7v7.8a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5z',
  sliders: 'M4 7h9M17 7h3M4 12h3M11 12h9M4 17h11M19 17h1M15 5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM9 10a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM17 15a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
  eye: 'M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12zM12 9.2a2.8 2.8 0 1 0 0 5.6 2.8 2.8 0 0 0 0-5.6z',
  eyeOff: 'M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12zM12 9.2a2.8 2.8 0 1 0 0 5.6 2.8 2.8 0 0 0 0-5.6zM4 4l16 16',
  mail: 'M6.5 5.5h11A3 3 0 0 1 20.5 8.5v7a3 3 0 0 1-3 3h-11a3 3 0 0 1-3-3v-7a3 3 0 0 1 3-3zM4.5 8l7.5 5.5L19.5 8',
  clock: 'M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17zM12 7.5V12l3 2',
  check: 'M5 12.5l4.5 4.5L19 7.5',
  alert: 'M12 4l9 15.5H3zM12 10v4.2M12 17v.01',
  wifioff: 'M3.5 9.5a13 13 0 0 1 4-2.4M20.5 9.5a13 13 0 0 0-8.5-3.4M6.5 13a8 8 0 0 1 2.7-1.8M17.5 13a8 8 0 0 0-3.4-1.9M9.5 16.3a4 4 0 0 1 5 0M12 19.5v.01M4 4l16 16',
  refresh: 'M19.5 12a7.5 7.5 0 0 1-13 5.1M4.5 12a7.5 7.5 0 0 1 13-5.1M17.5 3.5v4h-4M6.5 20.5v-4h4',
  spinner: 'M12 3.5a8.5 8.5 0 1 1-8.5 8.5',
} as const

export type IconName = keyof typeof PATHS

export function Icon({ name, size = 20, className }: { name: IconName; size?: number; className?: string }) {
  return (
    <svg
      className={className ? `ic ${className}` : 'ic'}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d={PATHS[name]} />
    </svg>
  )
}
