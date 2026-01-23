FROM rust:1.93.0-bookworm as build-env
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

FROM debian:bookworm-slim

WORKDIR /

RUN sed -i -e's/ main/ main contrib non-free/g' /etc/apt/sources.list.d/debian.sources && \
	apt-get update && apt-get install -y \
	libx11-6 libxcursor1 libx11-xcb1 libxi6 libxkbcommon-x11-0 \
	libgl1 libgl1-mesa-dri libgl1-nvidia-glvnd-glx \
	&& rm -rf /var/lib/apt/lists/*

COPY --chown=root:root --from=build-env \
	/usr/src/eew-renderer/CREDITS \
	/usr/src/eew-renderer/LICENSE \
	/usr/share/licenses/eew-renderer/

COPY --chown=root:root --from=build-env \
	/usr/src/eew-renderer/target/release/renderer \
	/usr/bin/eew-renderer

CMD ["/usr/bin/eew-renderer"]
