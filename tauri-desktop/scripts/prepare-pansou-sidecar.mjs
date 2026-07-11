import { chmodSync, existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
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
const repoRoot = resolve(fileURLToPath(new URL('..', import.meta.url)))
const outputDir = join(repoRoot, 'src-tauri', 'binaries')
let tempDir = ''

function getCliTargetValue() {
  for (let index = 2; index < process.argv.length; index += 1) {
    const arg = process.argv[index]
    if (arg === '--target' || arg === '--targets') {
      return process.argv[index + 1] || ''
    }
    if (arg.startsWith('--target=')) {
      return arg.slice('--target='.length)
    }
    if (arg.startsWith('--targets=')) {
      return arg.slice('--targets='.length)
    }
  }
  return ''
}

function resolveTargetKeys() {
  const requestedValue = process.env.PANSOU_TARGETS || getCliTargetValue() || hostKey
  const requestedItems = requestedValue
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
  const keys = requestedItems.includes('all') ? Object.keys(targets) : requestedItems
  const selected = []
  for (const key of keys) {
    const matchedKey = targets[key] ? key : Object.keys(targets).find((item) => targets[item].triple === key)
    if (!matchedKey) {
      throw new Error(`Unsupported PanSou sidecar target: ${key}`)
    }
    if (!selected.includes(matchedKey)) {
      selected.push(matchedKey)
    }
  }
  return selected
}

function sidecarOutput(target) {
  return join(outputDir, `pansou-sidecar-${target.triple}${target.ext}`)
}

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

function hasPreparedSidecar(file) {
  if (!existsSync(file)) return false
  const content = readFileSync(file)
  return !content.includes('PanSou sidecar placeholder')
}

function resolveGoLdflags(target) {
  const flags = ['-s', '-w']
  if (target.goos === 'windows') {
    flags.push('-H=windowsgui')
  }
  return flags.join(' ')
}

try {
  const targetKeys = resolveTargetKeys()
  if (!targetKeys.length) {
    console.log(`PanSou sidecar is not prepared for ${hostKey}. Skipping.`)
  } else if (!hasCommand('go')) {
    mkdirSync(outputDir, { recursive: true })
    for (const key of targetKeys) {
      const output = sidecarOutput(targets[key])
      if (hasPreparedSidecar(output)) {
        console.log(`Go is not installed. Keeping existing PanSou sidecar: ${output}`)
        continue
      }
      writeFileSync(output, '#!/bin/sh\nprintf "PanSou sidecar placeholder: Go toolchain is missing.\\n" >&2\nexit 1\n')
      chmodSync(output, 0o755)
      console.log(`Go is not installed. Wrote placeholder sidecar for local checks: ${output}`)
    }
  } else {
    mkdirSync(outputDir, { recursive: true })
    tempDir = mkdtempSync(join(tmpdir(), 'sui-frame-pansou-'))
    const sourceDir = join(tempDir, 'pansou')
    run('git', ['clone', '--depth', '1', '--branch', PANSOU_REF, PANSOU_REPO, sourceDir])
    for (const key of targetKeys) {
      const target = targets[key]
      const output = sidecarOutput(target)
      run('go', ['build', '-trimpath', '-ldflags', resolveGoLdflags(target), '-o', output, '.'], {
        cwd: sourceDir,
        env: {
          ...process.env,
          CGO_ENABLED: '0',
          GOOS: target.goos,
          GOARCH: target.goarch
        }
      })
      console.log(`Prepared PanSou sidecar: ${output}`)
    }
  }
} finally {
  if (tempDir) {
    rmSync(tempDir, { recursive: true, force: true })
  }
}
