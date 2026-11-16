import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('..', import.meta.url));
const test = process.argv.includes('--test-profile');
const bundle = process.argv.includes('--bundle');
const env = { ...process.env };
if (test) env.CARGO_TARGET_DIR = join(root, 'target/test-profile');
const args = ['build', '--target', 'x86_64-pc-windows-msvc'];
if (test) args.push('--config', join(root, 'src-tauri/tauri.test.conf.json'));
if (process.platform !== 'win32') args.push('--runner', 'cargo-xwin');
args.push(...(bundle ? ['--bundles', 'nsis'] : ['--no-bundle']));
args.push(...process.argv.slice(2).filter((arg) => !['--test-profile', '--bundle'].includes(arg)));
const result = spawnSync(
  process.execPath,
  [join(root, 'node_modules/@tauri-apps/cli/tauri.js'), ...args],
  { cwd: root, env, stdio: 'inherit' },
);
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
