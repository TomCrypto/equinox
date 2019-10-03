# Equinox

Equinox is a WebGL2 path tracer written in modern Rust targeting WebAssembly. Its goal is to provide me with an efficient testbed to play around with new computer graphics ideas and generate pretty renders.

The graphics back-end will be upgraded to WebGPU when this technology matures, in the meantime it is WebGL2-only. The rationale for this is simple: the path tracing algorithm is compute-intensive enough that you need a reasonably fast GPU to achieve interactivity, and therefore probably also have support for WebGL2. I also expect good speedups from WebGPU, a number of features require compute-shader functionality which is currently poorly emulated with fragment shaders.

## Building

Use `wasm-pack` (more info [here](https://github.com/rustwasm/wasm-pack)) to build the project as a WebAssembly module:

    wasm-pack build [--release]

The `viewer` project can also automatically build the module on any code change:

    cd viewer && npm install && npm run serve

## License

This software is licensed under the MIT license.

## Gallery


