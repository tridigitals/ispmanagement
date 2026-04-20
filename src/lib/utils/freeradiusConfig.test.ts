import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '../../..');

function readRepoFile(relativePath: string): string {
  return readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

describe('freeradius deployment config', () => {
  it('maps SQL auth lookups to the incoming RADIUS username', () => {
    const sqlTemplate = readRepoFile('deploy/freeradius/raddb/mods-available/sql.template');

    expect(sqlTemplate).toContain('sql_user_name = "%{User-Name}"');
    expect(sqlTemplate).toContain("a.username = '%{SQL-User-Name}'");
  });

  it('disables startup SQL client loading when dynamic clients are enabled', () => {
    const sqlTemplate = readRepoFile('deploy/freeradius/raddb/mods-available/sql.template');

    expect(sqlTemplate).toContain('read_clients = no');
    expect(sqlTemplate).not.toContain('client_query = ');
  });

  it('health-checks the config path that exists in the built image', () => {
    const compose = readRepoFile('docker-compose.radius.yml');

    expect(compose).toContain(
      `test: ['CMD-SHELL', 'test -f /etc/freeradius/mods-available/sql']`,
    );
  });

  it('does not keep a static pilot-router fallback in clients.conf', () => {
    const clientsConf = readRepoFile('deploy/freeradius/raddb/clients.conf');

    expect(clientsConf).not.toContain('client mikrotik-test');
    expect(clientsConf).not.toContain('103.190.112.210');
  });

  it('forces Message-Authenticator validation in the runtime config', () => {
    const entrypoint = readRepoFile('deploy/freeradius/docker-entrypoint.sh');

    expect(entrypoint).toContain('require_message_authenticator = yes');
    expect(entrypoint).toContain('sed -i');
  });

  it('ships a repo-managed dynamic-clients site', () => {
    const dynamicClientsSite = readRepoFile(
      'deploy/freeradius/raddb/sites-available/dynamic-clients',
    );

    expect(dynamicClientsSite).toContain('server dynamic_clients');
    expect(dynamicClientsSite).toContain('Packet-Src-IP-Address');
    expect(dynamicClientsSite).toContain('FreeRADIUS-Client-Require-MA');
  });

  it('documents that NAS mapping edits apply without a normal restart', () => {
    const readme = readRepoFile('deploy/freeradius/README.md');

    expect(readme).toContain('without restarting the container');
    expect(readme).not.toContain('adding a brand-new router/NAS mapping usually requires');
  });

  it('ships a dedicated helper script for restarting freeradius', () => {
    const script = readRepoFile('scripts/restart-freeradius.sh');
    const envExample = readRepoFile('deploy/systemd/server.env.example');
    const readme = readRepoFile('deploy/freeradius/README.md');

    expect(script).toContain('docker compose -f');
    expect(script).toContain('restart "$SERVICE_NAME"');
    expect(envExample).toContain('scripts/restart-freeradius.sh');
    expect(readme).toContain('scripts/restart-freeradius.sh');
  });
});
