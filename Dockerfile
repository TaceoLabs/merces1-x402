FROM rust:1.95 AS builder
WORKDIR /app

ARG SERVICE_NAME
RUN test -n "$SERVICE_NAME" || (echo "ERROR: SERVICE_NAME is required" && exit 1)

# Build application
COPY . .
ARG GIT_HASH
ENV GIT_HASH=$GIT_HASH

RUN cargo build --release --package $SERVICE_NAME
RUN mv target/release/$SERVICE_NAME /app/bin

# Build healthcheck
FROM rust:1.95 AS healthcheck-builder
RUN cargo install simple-web-healthcheck

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
ARG SERVICE_NAME
ARG GIT_HASH
LABEL org.opencontainers.image.revision=$GIT_HASH
LABEL org.opencontainers.image.vendor=TACEO
LABEL org.opencontainers.image.source=https://github.com/TaceoLabs/Merces1_updated
LABEL org.opencontainers.image.description=$SERVICE_NAME
WORKDIR /app
# copy healthcheck 
COPY --from=healthcheck-builder /usr/local/cargo/bin/simple-web-healthcheck /healthcheck

COPY --from=builder /app/bin /app/bin
# copy circom code + zk artifacts
COPY --from=builder /app/circom /app/circom

CMD [ "/app/bin" ]

