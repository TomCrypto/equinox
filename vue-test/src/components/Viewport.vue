<template>
  <canvas
    ref="canvas"
    tabindex="0"
    v-on:mousedown="enterCapture()"
    v-on:mouseup="leaveCapture()"
    v-on:mouseleave="leaveCapture()"
    v-on:mousemove="moveCamera($event)"
    v-on:keydown="pressKey($event.key)"
    v-on:keyup="releaseKey($event.key)"
  />
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebDevice, WebScene } from "equinox";

@Component
export default class Viewport extends Vue {
  @Prop() private equinox!: typeof import("equinox");
  @Prop() private scene!: WebScene;

  private device!: WebDevice;

  private keys: { [x: string]: boolean } = {};
  private theta: number = Math.PI / 2;
  private phi: number = Math.PI / 2;
  private mouseMoved: boolean = false;

  private captured: boolean = false;

  get canvas(): HTMLCanvasElement {
    return this.$refs.canvas as HTMLCanvasElement;
  }

  private pressKey(key: string) {
    if (!this.captured) {
      return;
    }

    this.keys[key] = true;
  }

  private releaseKey(key: string) {
    delete this.keys[key];
  }

  private moveCamera(event: MouseEvent) {
    if (!this.captured) {
      return;
    }

    this.phi += -event.movementX * 0.001;
    this.theta += -event.movementY * 0.001;

    if (this.theta > Math.PI - 0.01) {
      this.theta = Math.PI - 0.01;
    }

    if (this.theta < 0.01) {
      this.theta = 0.01;
    }

    this.mouseMoved = true;
  }

  private enterCapture() {
    this.canvas.requestPointerLock();
    this.captured = true;
  }

  private leaveCapture() {
    document.exitPointerLock();
    this.captured = false;
  }

  mounted() {
    this.device = new this.equinox.WebDevice(
      this.canvas.getContext("webgl2", {
        alpha: false,
        antialias: false,
        depth: false,
        premultipliedAlpha: false,
        stencil: false
      })
    );

    this.canvas.focus();

    const loop = () => {
      let forward = 0;
      let sideways = 0;

      if (this.keys["w"]) {
        forward += 1.0;
      }

      if (this.keys["s"]) {
        forward -= 1.0;
      }

      if (this.keys["a"]) {
        sideways -= 1.0;
      }

      if (this.keys["d"]) {
        sideways += 1.0;
      }

      if (forward != 0 || sideways != 0) {
        this.scene.move_camera(-forward * 0.1, -sideways * 0.1);
      }

      if (this.mouseMoved) {
        let x = Math.sin(this.theta) * Math.cos(this.phi);
        let z = Math.sin(this.theta) * Math.sin(this.phi);
        let y = Math.cos(this.theta);

        this.scene.set_camera_direction(x, y, z);

        this.mouseMoved = false;
      }

      if (this.canvas.clientWidth == 0 || this.canvas.clientHeight == 0) {
        requestAnimationFrame(loop);
        return; // canvas not ready
      }

      this.canvas.width = this.canvas.clientWidth;
      this.canvas.height = this.canvas.clientHeight;

      this.scene.set_raster_dimensions(this.canvas.width, this.canvas.height);

      this.device.update(this.scene);
      this.device.refine();
      this.device.render();

      requestAnimationFrame(loop);
    };

    requestAnimationFrame(loop);
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped>
canvas {
  width: 100vw;
  height: 100vh;
  background-color: black;
  display: block;
}
</style>
