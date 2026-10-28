# syntax=docker/dockerfile:1
# Build from inside server/: docker build -t charlotte-server:local .
FROM golang:1.23-alpine AS build
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /out/charlotte-server ./cmd/server \
    && mkdir -p /out/data

# No Go toolchain, shell or package manager in the final image. Non-root,
# with CA certificates for Supabase and other outbound HTTPS.
FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=build --chown=65532:65532 /out/charlotte-server /usr/local/bin/charlotte-server
COPY --from=build --chown=65532:65532 /out/data /app/data
ENV CHAT_USERNAMES_FILE=/app/data/usernames.log \
    CHAT_ROOMS_HLL_FILE=/app/data/rooms.hll \
    CHAT_USERS_HLL_FILE=/app/data/users.hll \
    AUTOCERT_CACHE_DIR=/app/data/autocert-cache
EXPOSE 8080 50051
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/charlotte-server"]
