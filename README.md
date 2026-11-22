# eew-renderer

<img width="1024" height="768" alt="image" src="https://github.com/user-attachments/assets/45b3752c-1802-4027-bdfc-a31b8b77510f" />

<!-- ![image](https://github.com/user-attachments/assets/9c51f47f-21ca-45b2-9e57-0b187bb96ff6) -->
<!-- ![image](https://github.com/user-attachments/assets/01c2159e-8237-41e4-b0ea-1afb49fa634a) -->
<!-- ![image](https://github.com/user-attachments/assets/b273798f-1410-44cc-a82b-ba9063d69289) -->
<!-- ![screenshot-1](https://github.com/EEWBot/eew-renderer/assets/11992915/058c05c4-93c9-41ba-858f-4ae297ae6efd) -->

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/EEWBot/eew-renderer)

eew-renderer is an HTTP server that renders earthquake intensity maps and tsunami forecast maps as images.
All rendering parameters are encoded into the request URL, so an image can be embedded anywhere a URL is accepted.

## Getting Started

### Docker

```sh
docker run --rm -p 3000:3000 -e HMAC_KEY=your-secret ghcr.io/eewbot/eew-renderer:master
```

Rendering is done off-screen on a GPU through Vulkan, so the container needs access to one.
To use an NVIDIA GPU, add `--gpus all` (requires the NVIDIA Container Toolkit).

If no hardware GPU is available, the server refuses to start.
To render on the CPU instead (slower), add `-e ALLOW_SOFTWARE_RENDERING=true`.

### Build from source

Requirements:

- Rust toolchain (the version is pinned in [`rust-toolchain`](rust-toolchain))
- `protoc` (Protocol Buffers compiler). If it is not on `PATH`, set the `PROTOC` environment variable to its path.
- The `assets/eew-renderer-proto` submodule

```sh
git clone --recursive https://github.com/EEWBot/eew-renderer
cd eew-renderer
cargo run --release -- --hmac-key your-secret
```

Rendering is done off-screen; no window is opened.
The GPU adapters found are logged at startup, and the most suitable one is selected automatically.

Open `http://localhost:3000/` to check that the server is running.

## Requesting Images

Send a `GET` request to `/<payload>` (an optional `.webp` suffix is accepted). The response is an `image/webp` image.

The payload is a Protocol Buffers message (see [`eew-renderer-proto`](https://github.com/EEWBot/eew-renderer-proto)) signed with HMAC-SHA1 using `HMAC_KEY`, and encoded in Base32768 (or legacy Base65536).
The following message types are supported:

| Type | Content |
|------|---------|
| `quake-prefecture-v0` | Seismic intensity by area, with the epicenter |
| `tsunami-forecast-v0` / `tsunami-forecast-v1` | Tsunami warnings, advisories and forecasts by area |

The easiest way to build URLs is [eew-renderer-url](https://github.com/EEWBot/eew-renderer-url):

```sh
URL=$(eew-renderer-url encode \
  --prefix http://localhost:3000/ \
  base32768 --hmac-key your-secret quake-prefecture-v0 \
    --time "2024-01-01T07:10:00Z" \
    --epicenter 37.5,137.2 \
    --three 350 --five-plus 380 --seven 390)

curl -o quake.webp "$URL"
```

For more examples, see [`.github/workflows/rendering.yml`](.github/workflows/rendering.yml).

Requests with an invalid signature are rejected with `401 Unauthorized`.
For local development, `--bypass-hmac` disables the check. **Never use it in production.**

## Configuration

Every option can be given as a command line flag or as an environment variable.

| Flag / Environment variable | Default | Description |
|------|---------|-------------|
| `--hmac-key` / `HMAC_KEY` | (empty) | Key used to verify request signatures |
| `--listen` / `LISTEN` | `0.0.0.0:3000` | Address to listen on |
| `--instance-name` / `INSTANCE_NAME` | `[not specified]` | Name returned in the `X-Instance-Name` response header |
| `--gpu-adapter-index` / `GPU_ADAPTER_INDEX` | (automatic) | Index of the GPU adapter to use, as listed in the startup log |
| `--allow-software-rendering` / `ALLOW_SOFTWARE_RENDERING` | `false` | Allow CPU renderers such as lavapipe. Without this, the server exits if no hardware GPU is found |
| `--image-cache-capacity` / `IMAGE_CACHE_CAPACITY` | `512` | Number of rendered images kept in memory |
| `--minimum-response-interval` / `MINIMUM_RESPONSE_INTERVAL` | `200ms` | Minimum interval between responses |
| `--client-ip-source` / `CLIENT_IP_SOURCE` | `ConnectInfo` | Where to read the client IP from. Set e.g. `CfConnectingIp` behind a proxy ([details](https://docs.rs/axum-client-ip/1.0.0/axum_client_ip/index.html#configurable-vs-specific-extractors)) |
| `--bypass-hmac` / `BYPASS_HMAC` | `false` | Skip signature verification (development only) |

Log verbosity can be controlled with `RUST_LOG` (e.g. `RUST_LOG=info`).

### Intensity stations

The list of seismic intensity stations can be replaced without rebuilding.

| Flag / Environment variable | Default | Description |
|------|---------|-------------|
| `--intensity-stations-source` / `INTENSITY_STATIONS_SOURCE` | `embedded` | `embedded`, `file:<PATH>` or `http(s)://<URL>` |
| `--intensity-stations-poll-interval` / `INTENSITY_STATIONS_POLL_INTERVAL` | `60s` | Polling interval for the HTTP source |
| `--intensity-stations-http-header` / `INTENSITY_STATIONS_HTTP_HEADER` | | Extra request header in `NAME: VALUE` form. Repeat the flag, or separate with newlines in the environment variable |
| `--intensity-stations-cf-access-client-id` / `INTENSITY_STATIONS_CF_ACCESS_CLIENT_ID` | | Cloudflare Access service token ID (must be set together with the secret) |
| `--intensity-stations-cf-access-client-secret` / `INTENSITY_STATIONS_CF_ACCESS_CLIENT_SECRET` | | Cloudflare Access service token secret |

- `embedded`: uses [`assets/intensity_stations.json`](assets/intensity_stations.json) bundled into the binary.
- `file:<PATH>`: loads the file at startup and reloads it automatically when it changes.
- `http(s)://<URL>`: fetches the file at startup and polls it periodically (using `ETag` when available).

The file must use the
[`jma-station-master` schema version 1](https://github.com/EEWBot/jma-station-master#%E5%87%BA%E5%8A%9B).
If the initial load fails, the server exits.

### Capturing a frame with RenderDoc

For debugging, headless frames can be captured with [RenderDoc](https://renderdoc.org/) and inspected draw call by draw call.
When built with the `renderdoc-capture` feature (off by default), every rendered frame is captured automatically.
Requests served from the image cache are not rendered, so they produce no capture.

wgpu only enables RenderDoc when its `wgpu-hal` crate is built with debug assertions.
Debug builds work as is; for release builds, enable them for `wgpu-hal` alone:

```sh
cargo build --release --features renderdoc-capture --config 'profile.release.package.wgpu-hal.debug-assertions=true'
```

Launch the binary from RenderDoc (Launch Application), then send requests.
Each capture covers the whole `render` call, including command recording, submission, waiting for the GPU and readback.

## Compatibility

Rendering uses [wgpu](https://wgpu.rs/), and a hardware GPU is required by default.

| Platform | Backend | Status |
|----------|---------|--------|
| Linux | Vulkan | Supported |
| Windows | Direct3D 12, Vulkan | Supported |
| macOS | Metal | Expected to work, but not tested |

OpenGL is used only as a last resort when no other backend is available.
The backends to consider can be restricted with the `WGPU_BACKEND` environment variable (e.g. `WGPU_BACKEND=vulkan`).

## License

eew-renderer source code is licensed under the [MIT License](LICENSE).

This project also uses geographic data provided by third parties.
Those datasets and data derived from them are subject to their respective
terms of use. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for details.
