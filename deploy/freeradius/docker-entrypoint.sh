#!/bin/sh
set -eu

if [ -d "/etc/freeradius" ] && [ -f "/etc/freeradius/radiusd.conf" ]; then
  RADDIR="/etc/freeradius"
else
  RADDIR="/etc/freeradius/3.0"
fi
SQL_TEMPLATE="${RADDIR}/mods-available/sql.template"
SQL_CONFIG="${RADDIR}/mods-available/sql"
RADIUSD_CONFIG="${RADDIR}/radiusd.conf"

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
mkdir -p "${RADDIR}/sites-enabled"

cp "$SQL_TEMPLATE" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_HOST__|${RADIUS_DB_HOST}|g" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_PORT__|${RADIUS_DB_PORT}|g" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_NAME__|${RADIUS_DB_NAME}|g" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_USER__|${RADIUS_DB_USER}|g" "$SQL_CONFIG"
sed -i "s|__RADIUS_DB_PASSWORD__|${RADIUS_DB_PASSWORD}|g" "$SQL_CONFIG"
sed -i 's/require_message_authenticator = auto/require_message_authenticator = yes/' "$RADIUSD_CONFIG"

if [ ! -e "${RADDIR}/mods-enabled/sql" ]; then
  ln -s ../mods-available/sql "${RADDIR}/mods-enabled/sql"
fi

if [ ! -e "${RADDIR}/sites-enabled/dynamic-clients" ]; then
  ln -s ../sites-available/dynamic-clients "${RADDIR}/sites-enabled/dynamic-clients"
fi

# This managed-RADIUS deployment only needs PAP/PPPoE auth. The upstream image
# enables EAP by default, but our trimmed server config does not define the
# corresponding Auth-Type sections, which causes startup validation to fail.
rm -f "${RADDIR}/mods-enabled/eap"

exec "$@"
