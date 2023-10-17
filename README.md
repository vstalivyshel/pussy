### WIP
[GlslViewer](https://github.com/patriciogonzalezvivo/glslViewer) clone.  

### Progress:
- [X] Hot reloading and syntax error reporting (to stdout).
- [X] Global bindings (will add useful bindings as needed).
  - `TIME` in seconds (f32) from the start of the renderer.
- [X] Record and save shader output as an image/video.
  - F5 will 'screenshot' the current frame and save it as .png file.
  - F6 will start recording frames. Pressing it again stops recording.
  - F7 will save the recorded frames as .mp4 file.
- [ ] GLSL support.
- [ ] Little preprocessor (mostly for including files and configuring the renderer).
- [ ] Load images and/or videos.

### All credits to:
- [GlslViewer](https://github.com/patriciogonzalezvivo/glslViewer)
- [ShaderToy](https://www.shadertoy.com/)
- [Code sample. Thx](https://github.com/compute-toys/wgpu-compute-toy)
- [Code sample. Thx](https://github.com/adamnemecek/shadertoy)

#### What with the name?
Funny compiler output:
```
    Checking pussy v0.1.0 (..)
    Finished dev [unoptimized + debuginfo] target(s) in 29.21s
```
