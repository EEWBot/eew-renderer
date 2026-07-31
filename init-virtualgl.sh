#!/bin/bash

# depName=VirtualGL/virtualgl
VIRTUALGL_VERSION="3.1.4"

apt-get install -y wget

case "$(uname -m)" in
	x86_64) ARCH="amd64" ;;
	aarch64) ARCH="arm64" ;;
	*) echo "Unsupported architecture: $(uname -m)" >&2; exit 1;;
esac

wget \
	"https://github.com/VirtualGL/virtualgl/releases/download/${VIRTUALGL_VERSION}/virtualgl_${VIRTUALGL_VERSION}_${ARCH}.deb" \
	-O "/virtualgl_${VIRTUALGL_VERSION}_${ARCH}.deb"

apt-get install -y "/virtualgl_${VIRTUALGL_VERSION}_${ARCH}.deb"

rm "/virtualgl_${VIRTUALGL_VERSION}_${ARCH}.deb"

apt-get purge -y wget
