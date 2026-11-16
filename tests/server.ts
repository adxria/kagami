import type { Plugin } from 'vite';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';
import { resolve } from 'node:path';

// One real Rust/SQLite process per browser session. The server is loaded only by
// Vite's test mode, never by a production build or the desktop adapter.
export function testServer(): Plugin {
  const drivers = new Map<
    string,
    { child: ChildProcessWithoutNullStreams; queue: ((value: unknown) => void)[] }
  >();
  return {
    name: 'rust-test-transport',
    configureServer(server) {
      server.httpServer?.on('close', () => {
        for (const { child } of drivers.values()) child.kill();
      });
      server.middlewares.use('/__test__/rpc', async (req, res) => {
        try {
          const id = String(req.headers['x-kagami-session']);
          if (!/^[\w-]{1,80}$/.test(id)) throw new Error('Invalid session');
          let driver = drivers.get(id);
          if (!driver) {
            const child = spawn(resolve('target/debug/kagami-driver'), [], { stdio: 'pipe' });
            driver = { child, queue: [] };
            drivers.set(id, driver);
            const d = driver;
            createInterface({ input: child.stdout }).on('line', (line) =>
              d.queue.shift()?.(JSON.parse(line)),
            );
            child.stderr.on('data', (data) => process.stderr.write(data));
          }
          let body = '';
          for await (const chunk of req) body += chunk;
          const result = await new Promise((done) => {
            driver!.queue.push(done);
            driver!.child.stdin.write(body + '\n');
          });
          res.setHeader('content-type', 'application/json');
          res.end(JSON.stringify(result));
        } catch (error) {
          res.statusCode = 500;
          res.end(JSON.stringify({ error: { message: String(error) } }));
        }
      });
    },
  };
}
