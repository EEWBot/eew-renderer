# EEW Renderer
<img width="1024" height="768" alt="image" src="https://github.com/user-attachments/assets/45b3752c-1802-4027-bdfc-a31b8b77510f" />

<!-- ![image](https://github.com/user-attachments/assets/9c51f47f-21ca-45b2-9e57-0b187bb96ff6) -->
<!-- ![image](https://github.com/user-attachments/assets/01c2159e-8237-41e4-b0ea-1afb49fa634a) -->
<!-- ![image](https://github.com/user-attachments/assets/b273798f-1410-44cc-a82b-ba9063d69289) -->
<!-- ![screenshot-1](https://github.com/EEWBot/eew-renderer/assets/11992915/058c05c4-93c9-41ba-858f-4ae297ae6efd) -->

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
