<template>
  <div>
    <h2>Shortcuts</h2>
    <table style="width:100%">
      <tr>
        <td>Toggle renderer status bar</td>
        <td>Shift + O</td>
      </tr>
      <tr>
        <td>Force WebGL context loss (for debugging)</td>
        <td>Shift + K</td>
      </tr>
      <tr>
        <td>Restore force-lost WebGL context (for debugging)</td>
        <td>Shift + L</td>
      </tr>
    </table>
    <h2>Camera Controls</h2>
    <p>
      All camera interaction is done when the cursor is captured by left- or right-clicking
      on the canvas.
    </p>
    <table style="width:100%">
      <tr>
        <td>Look around</td>
        <td>Mouse movement</td>
      </tr>
      <tr>
        <td>Basic movement</td>
        <td>WASD</td>
      </tr>
      <tr>
        <td>Pan upwards</td>
        <td>Q</td>
      </tr>
      <tr>
        <td>Pan downwards</td>
        <td>Z</td>
      </tr>
      <tr>
        <td>Adjust movement speed</td>
        <td>Scroll Wheel</td>
      </tr>
    </table>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {}
</script>

<style scoped></style>
