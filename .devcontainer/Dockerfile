FROM mcr.microsoft.com/devcontainers/javascript-node:1-22-bookworm

ARG USERNAME=vscode
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        curl \
        ca-certificates \
        git \
        unzip \
        libssl-dev \
        libwebkit2gtk-4.1-dev \
        libgtk-3-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libsecret-1-dev \
        libxdo-dev \
        libxkbcommon-dev \
        libasound2-dev \
        libdbus-1-dev \
        libglib2.0-dev \
        libpango1.0-dev \
        libatk1.0-dev \
        libxrandr-dev \
        libxcursor-dev \
        libxinerama-dev \
        libxi-dev \
        libxext-dev \
        libxfixes-dev \
        libxdamage-dev \
        libx11-dev \
        libdrm-dev \
        libgbm-dev \
        libinput-dev \
        libwayland-dev \
        libxkbregistry-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://www.rust-lang.org/tools/install.sh | sh -s -- -y --profile minimal
RUN curl -fsSL https://bun.sh/install | bash

RUN mkdir -p /home/${USERNAME}/.bun /tmp/bun-cache /tmp/npm-cache && \
    chown -R ${USERNAME}:${USERNAME} /home/${USERNAME} /tmp/bun-cache /tmp/npm-cache

ENV PATH="/home/${USERNAME}/.cargo/bin:/home/${USERNAME}/.bun/bin:${PATH}"

USER ${USERNAME}
WORKDIR /workspaces

RUN rustup default stable && rustup component add rustfmt clippy
