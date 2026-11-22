FROM rust:1.98.1-trixie AS build-env
LABEL maintainer="yanorei32"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

WORKDIR /usr/src

RUN apt-get update && apt-get install -y protobuf-compiler

RUN cargo new eew-renderer

COPY LICENSE \
	Cargo.toml \
	asset-preprocessor/Cargo.toml \
	renderer/Cargo.toml \
	renderer-assets/Cargo.toml \
	renderer-types/Cargo.toml \
	Cargo.lock \
	/usr/src/eew-renderer/

WORKDIR /usr/src/eew-renderer
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN	cargo install cargo-license && cargo license \
	--authors \
	--do-not-bundle \
	--avoid-dev-deps \
	--avoid-build-deps \
	--filter-platform "$(rustc -vV | sed -n 's|host: ||p')" \
	> CREDITS

RUN cargo build --release
COPY . /usr/src/eew-renderer/
RUN touch  assets/* src/* && cargo build --release

FROM debian:trixie-slim

WORKDIR /

# libvulkan1: Vulkan loader
# libxext6: required by the NVIDIA Vulkan driver injected by the NVIDIA Container Toolkit
# mesa-vulkan-drivers: software renderer (lavapipe), used only with ALLOW_SOFTWARE_RENDERING=true
RUN apt-get update && apt-get install -y --no-install-recommends \
	libvulkan1 libxext6 mesa-vulkan-drivers \
	&& rm -rf /var/lib/apt/lists/*

ENV NVIDIA_VISIBLE_DEVICES=all
ENV NVIDIA_DRIVER_CAPABILITIES=graphics,utility

COPY --chown=root:root --from=build-env \
	/usr/src/eew-renderer/CREDITS \
	/usr/src/eew-renderer/LICENSE \
	/usr/src/eew-renderer/THIRD_PARTY_NOTICES.md \
	/usr/share/licenses/eew-renderer/

COPY --chown=root:root --from=build-env \
	/usr/src/eew-renderer/target/release/renderer \
	/usr/bin/eew-renderer

CMD ["/usr/bin/eew-renderer"]
