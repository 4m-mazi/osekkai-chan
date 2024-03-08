# syntax=docker/dockerfile:1.7.0@sha256:dbbd5e059e8a07ff7ea6233b213b36aa516b4c53c645f1817a4dd18b83cbea56

FROM --platform=$BUILDPLATFORM busybox:1.36.1-uclibc@sha256:7bfe9a36e5f3917297b1c39348f32a55483988476a5b0bdb5888a6bd9dedd783 AS env
ARG TARGETPLATFORM
RUN --mount=type=bind,source=artifact,target=artifact \
    if [ "$TARGETPLATFORM" = 'linux/amd64' ]; then\
      cp artifact/x86_64-unknown-linux-musl/binary .\
    ;elif [ "$TARGETPLATFORM" = 'linux/arm64' ]; then\
      cp artifact/aarch64-unknown-linux-musl/binary .\
    ;fi

FROM scratch as runner
COPY --from=env --chmod=755 binary .
ENTRYPOINT ["./binary"]
