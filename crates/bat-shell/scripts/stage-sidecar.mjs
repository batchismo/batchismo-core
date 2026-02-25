#!/usr/bin/env node
// Stage bat-agent binary as a Tauri externalBin sidecar with target-triple suffix.
// Usage: node stage-sidecar.mjs [dev|release]
import { execSync } from 'child_process';
import { copyFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const batShellDir = join(__dirname, '..');
const workspaceRoot = join(batShellDir, '..', '..');

const profile = process.argv[2] === 'release' ? 'release' : 'debug';
const triple = execSync('rustc --print host-tuple').toString().trim();
const ext = process.platform === 'win32' ? '.exe' : '';

const src = join(workspaceRoot, 'target', profile, `bat-agent${ext}`);
const destDir = join(batShellDir, 'binaries');
const dest = join(destDir, `bat-agent-${triple}${ext}`);

mkdirSync(destDir, { recursive: true });

if (!existsSync(src)) {
  console.error(`bat-agent not found at ${src}`);
  console.error(`Build it first: cargo build -p bat-agent${profile === 'release' ? ' --release' : ''}`);
  process.exit(1);
}

copyFileSync(src, dest);
console.log(`Staged: ${dest}`);
