#!/bin/sh
set -eu

RADDIR="/etc/freeradius/3.0"
SQL_TEMPLATE="${RADDIR}/mods-available/sql.template"
SQL_CONFIG="${RADDIR}/mods-available/sql"

required_vars="
RADIUS_DB_HOST
RADIUS_DB_PORT
RADIUS_DB_NAME
RADIUS_DB_USER
RADIUS_DB_PASSWORD
"

for var_name in $required_vars; do
  eval "var_value=\${$var_name:-}"
  if [ -z "$var_value" ]; then
    echo "Missing required env: $var_name" >&2
    exit 1
  fi
done

mkdir -p "${RADDIR}/mods-enabled"

cp "$SQL_TEMPLATE" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_HOST__|${RADIUS_DB_HOST}|g" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_PORT__|${RADIUS_DB_PORT}|g" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_NAME__|${RADIUS_DB_NAME}|g" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_USER__|${RADIUS_DB_USER}|g" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_PASSWORD__|${RADIUS_DB_PASSWORD}|g" "$SQL_CONFIG"

if [ ! -e "${RADDIR}/mods-enabled/sql" ]; then
  ln -s ../mods-available/sql "${RADDIR}/mods-enabled/sql"
fi

exec "$@"
