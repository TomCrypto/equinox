# Equinox

Equinox is a WebGL2 stochastic progressive photon mapper written in modern Rust targeting WebAssembly. Its goal is to provide me with an efficient playground to experiment with new computer graphics ideas and generate pretty renders. The renderer itself is completely safe Rust and forbids unsafe code.

The graphics back-end will be upgraded to WebGPU when this technology matures, in the meantime it is WebGL2-only. The rationale for this is simple: the photon mapping algorithm is compute-intensive enough that you need a reasonably fast GPU to achieve interactivity, and therefore probably also have support for WebGL2. I also expect good speedups from WebGPU, a number of features require compute-shader functionality which is currently being emulated using the conventional rendering pipeline.

## Building

Use `wasm-pack` (more info [here](https://github.com/rustwasm/wasm-pack)) to build the project as a WebAssembly module:

    wasm-pack build [--release]

The `viewer` front-end will use the built module in the `pkg` folder, it's recommended to `yarn link` it so that any `wasm-pack` builds will automatically trigger a front-end rebuild.

    cd viewer && yarn install && yarn serve

Remember to download the assets for local use by running the Makefile in the `assets` folder.

## License

This software is provided under the MIT license.

## Gallery
