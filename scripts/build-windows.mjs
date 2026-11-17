import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('..', import.meta.url));
const test = process.argv.includes('--test-profile');
const bundle = process.argv.includes('--bundle');
const env = { ...process.env };
if (bundle && process.platform !== 'win32') {
  const nsis = spawnSync('makensis', ['-VERSION'], { env, encoding: 'utf8' });
  if (nsis.error || nsis.status !== 0) {
    console.error(
      'Windows installer builds require the native NSIS compiler (makensis) on PATH.\n' +
        'Install NSIS first; see README.md → Cross-compiling from Linux or WSL.\n' +
        'Use npm run build:windows if you only need kagami.exe.',
    );
    if (nsis.error) console.error(nsis.error.message);
    else console.error(nsis.stderr || nsis.stdout);
    process.exit(1);
  }
}
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
