# Build stage
FROM rust:1.84-slim AS builder

# Install required system dependencies for static linking
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Determine target architecture automatically
ARG TARGETARCH
ENV TARGET_TRIPLE=""

# Set target triple and install target
RUN case "$TARGETARCH" in \
    "amd64") TARGET_TRIPLE="x86_64-unknown-linux-gnu" ;; \
    "arm64") TARGET_TRIPLE="aarch64-unknown-linux-gnu" ;; \
    *) echo "Unsupported architecture: $TARGETARCH" && exit 1 ;; \
    esac && \
    echo "Building for target: $TARGET_TRIPLE" && \
    rustup target add $TARGET_TRIPLE && \
    echo $TARGET_TRIPLE > /tmp/target_triple

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this will be cached unless Cargo.toml changes)
RUN TARGET_TRIPLE=$(cat /tmp/target_triple) && \
    cargo build --release --target "$TARGET_TRIPLE"
RUN rm src/main.rs

# Copy the actual source code
COPY src/ ./src/

# Copy static assets and templates
COPY static/ ./static/
COPY templates/ ./templates/

# Build the application with static linking
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN TARGET_TRIPLE=$(cat /tmp/target_triple) && \
    cargo build --release --target "$TARGET_TRIPLE" && \
    cp "target/$TARGET_TRIPLE/release/simple-docker-manager" /tmp/simple-docker-manager

# Runtime stage using scratch for minimal size
FROM scratch

# Build args for labels
ARG VERSION=0.1.0
ARG BUILD_DATE
ARG VCS_REF

# OCI Labels for better container registry UX
LABEL org.opencontainers.image.title="Simple Docker Manager"
LABEL org.opencontainers.image.description="A beautiful, lightweight Docker container management service with real-time metrics visualization"
LABEL org.opencontainers.image.version="${VERSION}"
LABEL org.opencontainers.image.authors="Simple Docker Manager Contributors"
LABEL org.opencontainers.image.url="https://github.com/OscillateLabsLLC/simple-docker-manager"
LABEL org.opencontainers.image.documentation="https://github.com/OscillateLabsLLC/simple-docker-manager#readme"
LABEL org.opencontainers.image.source="https://github.com/OscillateLabsLLC/simple-docker-manager"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.vendor="Simple Docker Manager"
LABEL org.opencontainers.image.created="${BUILD_DATE}"
LABEL org.opencontainers.image.revision="${VCS_REF}"

# Runtime labels
LABEL maintainer="Simple Docker Manager Contributors"
LABEL org.opencontainers.image.base.name="scratch"

# Copy CA certificates for HTTPS support
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy the statically compiled binary
COPY --from=builder /tmp/simple-docker-manager /usr/local/bin/simple-docker-manager

# Copy static assets and templates to the container
COPY --from=builder /app/static/ /app/static/
COPY --from=builder /app/templates/ /app/templates/

# Set the working directory for the application
WORKDIR /app

# Create a non-root user
# Note: For scratch images, we need to add the user in the final stage
# We use a high UID to avoid conflicts with host system users
USER 10001:10001

# Expose the default port
EXPOSE 3000

# Note: Health checks should be performed externally by hitting /health endpoint
# Example: curl -f http://localhost:3000/health

# Use the binary as the entrypoint
ENTRYPOINT ["/usr/local/bin/simple-docker-manager"] 