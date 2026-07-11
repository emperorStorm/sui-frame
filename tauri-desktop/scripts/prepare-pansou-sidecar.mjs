import { chmodSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const PANSOU_REPO = 'https://github.com/fish2018/pansou.git'
const PANSOU_REF = process.env.PANSOU_REF || 'v2.0'

const targets = {
  'darwin-arm64': {
    triple: 'aarch64-apple-darwin',
    goos: 'darwin',
    goarch: 'arm64',
    ext: ''
  },
  'win32-x64': {
    triple: 'x86_64-pc-windows-msvc',
    goos: 'windows',
    goarch: 'amd64',
    ext: '.exe'
  }
}

const hostKey = `${process.platform}-${process.arch}`
const target = targets[hostKey]

if (!target) {
  console.log(`PanSou sidecar is not prepared for ${hostKey}. Skipping.`)
  process.exit(0)
}

const repoRoot = resolve(fileURLToPath(new URL('..', import.meta.url)))
const outputDir = join(repoRoot, 'src-tauri', 'binaries')
const output = join(outputDir, `pansou-sidecar-${target.triple}${target.ext}`)
const tempDir = mkdtempSync(join(tmpdir(), 'sui-frame-pansou-'))
const sourceDir = join(tempDir, 'pansou')

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    stdio: 'inherit',
    ...options
  })
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed`)
  }
}

function hasCommand(command) {
  const result = spawnSync(command, ['version'], { stdio: 'ignore' })
  return result.status === 0
}

try {
  mkdirSync(outputDir, { recursive: true })
  if (!hasCommand('go')) {
    writeFileSync(output, '#!/bin/sh\nprintf "PanSou sidecar placeholder: Go toolchain is missing.\\n" >&2\nexit 1\n')
    chmodSync(output, 0o755)
    console.log(`Go is not installed. Wrote placeholder sidecar for local checks: ${output}`)
    process.exit(0)
  }
  run('git', ['clone', '--depth', '1', '--branch', PANSOU_REF, PANSOU_REPO, sourceDir])
  run('go', ['build', '-trimpath', '-ldflags', '-s -w', '-o', output, '.'], {
    cwd: sourceDir,
    env: {
      ...process.env,
      CGO_ENABLED: '0',
      GOOS: target.goos,
      GOARCH: target.goarch
    }
  })
  console.log(`Prepared PanSou sidecar: ${output}`)
} finally {
  rmSync(tempDir, { recursive: true, force: true })
}
