# Equinox

Equinox is a WebGL2 stochastic progressive photon mapper written in modern Rust targeting WebAssembly. Its goal is to provide me with an efficient playground to experiment with new computer graphics ideas and generate pretty renders. The renderer itself is completely safe Rust and forbids unsafe code.

<p align="center">
<img src="./doc/featured.png?raw=true" alt="Featured Render"/>
</p>

See the [gallery](gallery/README.md) for more pretty pictures, or check out the [live demo](https://tomcrypto.github.io/equinox/)! Note that the front-end is currently designed with desktops in mind and the renderer has not yet received much testing on mobile devices or Intel graphics cards.

## Features

- Stochastic progressive photon mapping
- Distance field geometries with CSG modifiers
- Physically accurate materials (including absorption)
- Triplanar texturing for arbitrary material attributes
- High quality image-based environment lighting
- Physically based, high quality lens flare module

All of these features are fully dynamic and editable in real-time with immediate feedback.

## Planned

- Triangle meshes (possibly replacing distance field geometries altogether)
- Additional materials
- Support for light sources other than environment lighting

The graphics back-end will be upgraded to WebGPU when this technology matures, in the meantime it is WebGL2-only. The rationale for this is simple: the photon mapping algorithm is compute-intensive enough that you need a reasonably fast GPU to achieve interactivity, and therefore probably also have support for WebGL2. I also expect good speedups from WebGPU, a number of features require compute-shader functionality which is currently being emulated using the traditional rendering pipeline.

## Building

Use `wasm-pack` (more info [here](https://github.com/rustwasm/wasm-pack)) to build the photon mapper as a WebAssembly module:

    wasm-pack build [--release]

The `viewer` front-end will use the built module in the `pkg` folder, it's suggested to `yarn link` it for development so that any `wasm-pack` builds will automatically trigger a front-end rebuild. To serve the front-end locally, run the following:

    cd viewer && yarn && yarn serve

You should download all assets (a few gigabytes total) for local use by running the Makefile in the `assets` folder.

## License

This software is provided under the MIT license.
