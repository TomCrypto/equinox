# Equinox

This is a WebGL2 path tracer written in Rust for WebAssembly. Its goal is to provide me with a testbed to try out and demo new ray-tracing related ideas and produce pretty pictures. The graphics back-end will be upgraded to WebGPU when this technology matures, in the meantime it is WebGL2-only. The rationale for this is simple: the path tracing algorithm is compute-intensive enough that you need a reasonably fast GPU to achieve interactivity, and therefore probably also have support for WebGL2. I expect good speedups from WebGPU, a number of features require compute-shader functionality which is currently emulated with fragment shaders.

## License

This software is licensed under the MIT license.
