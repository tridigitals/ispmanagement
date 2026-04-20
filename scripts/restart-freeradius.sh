#!/usr/bin/env bash
set -euo pipefail

WORKDIR="${MANAGED_RADIUS_RESTART_WORKDIR:-$(pwd)}"
COMPOSE_FILE="${MANAGED_RADIUS_COMPOSE_FILE:-docker-compose.radius.yml}"
SERVICE_NAME="${MANAGED_RADIUS_SERVICE_NAME:-freeradius}"

cd "$WORKDIR"

if [[ "${MANAGED_RADIUS_RESTART_DRY_RUN:-0}" == "1" ]]; then
  printf 'docker compose -f %q restart %q\n' "$COMPOSE_FILE" "$SERVICE_NAME"
  exit 0
fi

docker compose -f "$COMPOSE_FILE" restart "$SERVICE_NAME"
