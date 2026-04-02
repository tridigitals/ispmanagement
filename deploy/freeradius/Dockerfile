ARG FREERADIUS_BASE_IMAGE=freeradius/freeradius-server:latest
FROM ${FREERADIUS_BASE_IMAGE}

COPY raddb/ /etc/freeradius/3.0/
COPY docker-entrypoint.sh /usr/local/bin/isp-freeradius-entrypoint.sh

RUN chmod +x /usr/local/bin/isp-freeradius-entrypoint.sh

ENTRYPOINT ["/usr/local/bin/isp-freeradius-entrypoint.sh"]
CMD ["freeradius", "-f"]
