# The steward as an image: the unit as a package installs it, on an empty
# image, and nothing else. scripts/image.sh lays the unit out for every
# architecture from what scripts/build.sh made, and that is the context:
# TARGETARCH (amd64 or arm64) picks the tree. Nothing is compiled and
# nothing is run in the build, so neither architecture is emulated.
#
#   <arch>/rootfs/  the unit: the program, its face, its catalogue, its
#                   configuration and the licences; root's, and only read
#   <arch>/kept/    the directories the steward writes to: what it keeps
#                   (/var/lib/evo), its client socket (/run/evo), /tmp
#
# The image has one account, evo, numbered 10001, and no shell. The
# steward runs as that account, which owns what it writes to and nothing
# else.
FROM scratch
ARG TARGETARCH
COPY ${TARGETARCH}/rootfs/ /
COPY --chown=10001:10001 ${TARGETARCH}/kept/ /
USER 10001:10001

# The face, on every address the container has; which of them is reached
# from where is said when the container is started. The unit's code,
# which only a unit of the extended level asks, is read from the file
# named here, where the container is given one; and what the steward is
# believed by, with the usher's people beside it, is kept where the last
# names.
ENV EVO_HTTPS_LISTEN_ADDR=0.0.0.0:8443 \
    EVO_HTTPS_STATIC_DIR=/opt/evo/ui \
    EVO_PAIR_PRESEED_FILE=/etc/evo/unit-code \
    EVO_HTTPS_STATE_DIR=/var/lib/evo/state
EXPOSE 8443

# What the steward keeps outlives the container.
VOLUME /var/lib/evo

# Whether the unit is up, asked of the usher's own program, which is all
# the image has: the steward answers on its client socket and its
# listener takes a connection. Whatever runs the container goes by it.
HEALTHCHECK --interval=10s --timeout=5s --start-period=20s --retries=3 \
    CMD ["/opt/evo/bin/evo-usher-steward", "--healthy"]

ENTRYPOINT ["/opt/evo/bin/evo-usher-steward"]
