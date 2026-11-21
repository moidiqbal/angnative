// Cross-platform launcher for the bundled Gradle wrapper.
// Usage: node scripts/gradle.mjs <gradle tasks...>
import { spawnSync } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const androidDir = resolve(root, 'android')
const wrapper = resolve(androidDir, process.platform === 'win32' ? 'gradlew.bat' : 'gradlew')
const tasks = process.argv.slice(2)

// On Windows the batch path must be quoted for cmd.exe because workspace
// paths may contain spaces; Node wraps the whole command line for the shell.
const command = process.platform === 'win32' ? `"${wrapper}"` : wrapper

const result = spawnSync(command, tasks, {
  cwd: androidDir,
  stdio: 'inherit',
  shell: process.platform === 'win32'
})

if (result.error) {
  console.error(`Failed to launch ${wrapper}: ${result.error.message}`)
  process.exit(1)
}
process.exit(result.status ?? 1)
