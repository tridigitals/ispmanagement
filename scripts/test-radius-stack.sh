#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.radius.yml}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is not installed or not in PATH" >&2
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "docker compose is not available" >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "cannot talk to docker daemon" >&2
  echo "if needed, add your user to the docker group or run via sudo on the host" >&2
  exit 1
fi

echo "==> docker compose config"
docker compose -f "$COMPOSE_FILE" config >/dev/null

echo "==> building freeradius image"
docker compose -f "$COMPOSE_FILE" build freeradius

echo "==> starting stack"
docker compose -f "$COMPOSE_FILE" up -d

echo "==> waiting for radius-postgres health"
for _ in $(seq 1 30); do
  status="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' isp_radius_postgres 2>/dev/null || true)"
  if [ "$status" = "healthy" ]; then
    break
  fi
  sleep 2
done

echo "==> waiting for freeradius container"
for _ in $(seq 1 15); do
  status="$(docker inspect --format '{{.State.Status}}' isp_freeradius 2>/dev/null || true)"
  if [ "$status" = "running" ]; then
    break
  fi
  sleep 2
done

echo "==> compose ps"
docker compose -f "$COMPOSE_FILE" ps

echo "==> recent freeradius logs"
docker compose -f "$COMPOSE_FILE" logs --tail=80 freeradius

echo "==> recent radius-postgres logs"
docker compose -f "$COMPOSE_FILE" logs --tail=40 radius-postgres

echo "==> done"
