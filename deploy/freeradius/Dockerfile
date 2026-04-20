ARG FREERADIUS_BASE_IMAGE=freeradius/freeradius-server:latest
FROM ${FREERADIUS_BASE_IMAGE}

COPY raddb/ /etc/freeradius/
COPY docker-entrypoint.sh /usr/local/bin/isp-freeradius-entrypoint.sh

RUN chmod +x /usr/local/bin/isp-freeradius-entrypoint.sh
RUN rm -f /etc/freeradius/mods-enabled/eap

ENTRYPOINT ["/usr/local/bin/isp-freeradius-entrypoint.sh"]
CMD ["freeradius", "-f"]
