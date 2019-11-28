<template>
  <div>
    <h2>Equinox Renderer</h2>

    <p>
      This is a WebGL2 real-time stochastic photon mapper implemented using Rust
      WebAssembly. Like path tracing, it is progressive, meaning that
      the render starts out noisy and gradually improves over time, and it can
      produce highly accurate and pleasing images.
    </p>

    <h3>Quick Start</h3>

    <p>
      The application is divided into two panels, with the render output on one side
      and editing controls on the other. The editing controls are divided into tabs
      which each allow editing some aspect of the scene. Each scene also contains a camera from the perspective of which an image is rendered. To
      control the camera, hold down the left or right mouse button and use the mouse and keyboard:
    </p>

    <table style="width:100%">
      <tr>
        <td>Reorient the camera</td>
        <td>Mouse movement (FPS-like)</td>
      </tr>
      <tr>
        <td>Reposition the camera</td>
        <td>WASD, Q (up), Z (down)</td>
      </tr>
      <tr>
        <td>Adjust movement speed</td>
        <td>Scroll Wheel</td>
      </tr>
    </table>

    <p>
      A default scene with a few objects in it has been configured. Explore the editing options
      using this scene (a good starting point is to set an environment map for more interesting
      lighting) or load another scene of your choosing.
    </p>

    <h3>Changelog</h3>

    <ul>
      <li>
        <b>2019-11-25</b>: New UI
      </li>
    </ul>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {}
</script>

<style scoped></style>
