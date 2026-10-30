# Development
FROM node:26-bookworm AS web-development

WORKDIR /app

COPY package.json /app/

RUN npm install



FROM rust:bookworm AS api-development

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        clang \
        libclang-dev \
        libavcodec-dev \
        libavformat-dev \
        libavfilter-dev \
        libavdevice-dev \
        libavutil-dev \
        libswscale-dev \
        libswresample-dev \
        ca-certificates \
        curl \
        tar \
    && mkdir -p /tmp/pdfium /usr/local/lib \
    && curl -fsSL \
        https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F8076/pdfium-linux-x64.tgz \
        -o /tmp/pdfium.tar.gz \
    && tar -xzf /tmp/pdfium.tar.gz -C /tmp/pdfium \
    && cp /tmp/pdfium/lib/libpdfium.so /usr/local/lib/ \
    && ldconfig \
    && rm -rf /tmp/pdfium /tmp/pdfium.tar.gz \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-watch

WORKDIR /app

CMD ["cargo", "watch", "-x", "run"]


FROM rust:bookworm AS api-builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        clang \
        libclang-dev \
        libavcodec-dev \
        libavformat-dev \
        libavfilter-dev \
        libavdevice-dev \
        libavutil-dev \
        libswscale-dev \
        libswresample-dev \
        ca-certificates \
        curl \
        tar \
    && mkdir -p /tmp/pdfium /usr/local/lib \
    && curl -fsSL \
        https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F8076/pdfium-linux-x64.tgz \
        -o /tmp/pdfium.tar.gz \
    && tar -xzf /tmp/pdfium.tar.gz -C /tmp/pdfium \
    && cp /tmp/pdfium/lib/libpdfium.so /usr/local/lib/ \
    && ldconfig \
    && rm -rf /tmp/pdfium /tmp/pdfium.tar.gz \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/var/cache/cargo \
    CARGO_HOME=/var/cache/cargo \
    cargo build --release \
    && cp /app/target/release/fshare-backend /usr/local/bin/fshare-backend


FROM debian:bookworm-slim AS api-production

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libavcodec59 \
        libavformat59 \
        libavfilter8 \
        libavdevice59 \
        libavutil57 \
        libswscale6 \
        libswresample4 \
        ffmpeg \
    && rm -rf /var/lib/apt/lists/*

COPY --from=api-builder /usr/local/bin/fshare-backend /usr/local/bin/fshare-backend

# Copy PDFium from the builder instead of downloading it again.
COPY --from=api-builder /usr/local/lib/libpdfium.so /usr/local/lib/libpdfium.so

# Register the shared library path.
RUN ldconfig

CMD ["/usr/local/bin/fshare-backend"]
