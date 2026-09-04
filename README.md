# EEW Renderer

<img width="1024" height="768" alt="image" src="https://github.com/user-attachments/assets/45b3752c-1802-4027-bdfc-a31b8b77510f" />

<!-- ![image](https://github.com/user-attachments/assets/9c51f47f-21ca-45b2-9e57-0b187bb96ff6) -->
<!-- ![image](https://github.com/user-attachments/assets/01c2159e-8237-41e4-b0ea-1afb49fa634a) -->
<!-- ![image](https://github.com/user-attachments/assets/b273798f-1410-44cc-a82b-ba9063d69289) -->
<!-- ![screenshot-1](https://github.com/EEWBot/eew-renderer/assets/11992915/058c05c4-93c9-41ba-858f-4ae297ae6efd) -->

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/EEWBot/eew-renderer)

## Intensity Station Master

The intensity station master (`intensity_stations.json`) can be supplied at runtime:

| | |
| --- | --- |
| `--intensity-stations <PATH>` | Load the station master from an external JSON file. |
| `INTENSITY_STATIONS=<PATH>` | Same as above, via environment variable. |
| *(not specified)* | Use the JSON embedded at build time (`assets/intensity_stations.json`). |

Behaviour:

- **Startup failure is fatal.** If an external file is specified and it cannot be
  read, parsed or validated (malformed JSON, non-finite or out-of-range
  coordinates, duplicate `stationCode`, or an area present on the map but missing
  from the master), the renderer exits instead of falling back to the embedded data.
- **The external file is watched automatically.** Its parent directory is monitored,
  so both in-place edits and `write temporary file` → `rename` style atomic
  replacements are picked up. Changes are applied without restarting the process.
- **Runtime reload failure keeps the last-known-good master.** A reload is
  transactional: read → parse → validate → resolve must all succeed before the new
  data is swapped in. On failure the renderer logs the error, keeps serving the
  previously loaded master, and keeps running. This also covers the moment where the
  file temporarily does not exist during an atomic replacement.
- **Only the station master is reloadable.** Map geometry, area boundaries and
  prefecture boundaries are fixed at build time and are not affected.

Docker bind mount example:

```sh
docker run \
  -v /srv/eew/intensity_stations.json:/data/intensity_stations.json:ro \
  -e INTENSITY_STATIONS=/data/intensity_stations.json \
  ghcr.io/eewbot/eew-renderer
```

Note that some tools replace a bind-mounted file in a way the container cannot
observe; mounting the containing directory instead is the more robust option:

```sh
docker run \
  -v /srv/eew:/data:ro \
  -e INTENSITY_STATIONS=/data/intensity_stations.json \
  ghcr.io/eewbot/eew-renderer
```

## Compatibility

Basically, this project supports GL_VERSION >= 4.5 platforms.

The following environments are known to cause crashes due to errors:

```
GL_VENDOR: Intel
GL_RENDERER: Mesa Intel(R) Iris(R) Graphics 5100 (HSW GT3)
GL_VERSION: 4.6 (Core Profile) Mesa 26.0.1-arch1.1
```

```
ProgramCreation(LinkingError("error: Too many vertex shader image uniforms (1 > 0)\n"))
```

This is thought to be due to the fact that the Vertex Shader cannot use textures with uniforms, and there are no plans to fix this.

The workaround is to use an alternative GL implementation, such as LIBGL_ALWAYS_SOFTWARE.

## License

EEW Renderer source code is licensed under the [MIT License](LICENSE).

This project also uses geographic data provided by third parties.
Those datasets and data derived from them are subject to their respective
terms of use. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for details.
