# The core as an image: the static program on an empty image, and nothing
# else. scripts/image.sh builds it for both architectures from what
# scripts/build.sh made; the context is out/, and TARGETARCH (amd64 or
# arm64) picks the tree the program is copied from. Nothing is compiled
# and nothing is run in the build, so neither architecture is emulated.
FROM scratch
ARG TARGETARCH
COPY ${TARGETARCH}/bin/evo-usher-core /evo-usher-core
ENTRYPOINT ["/evo-usher-core"]
