import { spawnSync } from 'node:child_process';

const command = process.platform === 'win32' ? 'npx.cmd' : 'npx';
const result = spawnSync(
  command,
  ['node', 'scripts/native-graph-parity-vitest.mjs'],
  {
    stdio: 'inherit',
  },
);

process.exit(result.status ?? 1);
