#!/usr/bin/env bash

# dpkg --add-architecture $CROSS_DEB_ARCH && \
apt-get update && \
apt-get install -y \
    libssl-dev