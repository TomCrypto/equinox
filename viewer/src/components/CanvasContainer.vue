<template>
  <div class="container">
    <canvas
      class="canvas"
      v-bind:class="{ 'canvas-locked': isCameraLocked || isRenderPaused }"
      :style="canvasStyle"
      ref="canvas"
      tabindex="0"
      v-on:mousedown="enterCapture()"
      v-on:mouseup="leaveCapture()"
      v-on:mouseleave="leaveCapture()"
      v-on:mousemove="moveCamera($event)"
      v-on:wheel="onMouseWheel($event.deltaY)"
      v-on:keydown="pressKey($event.key)"
      v-on:keyup="releaseKey($event.key)"
      v-on:keypress="onKeyPress($event)"
      v-on:contextmenu="$event.preventDefault()"
    />

    <CanvasOverlay
      :assets-in-flight="assetsInFlight"
      :is-expensive-update="isExpensiveUpdate"
      :error-message="updateError"
    />

    <StatusBar
      v-if="showStatusBar"
      class="status-bar"
      :sppm-passes="sppmPasses"
      :sppm-photons="sppmPhotons"
      :is-context-lost="isContextLost"
      :width="canvasWidth"
      :height="canvasHeight"
      :vendor="contextVendor"
      :renderer="contextRenderer"
      :cpuFrameTime="cpuFrameTime"
      :gpuFrameTime="gpuFrameTime"
      :syncInterval="syncInterval"
    />

    <Toolbar
      :on-save-render="onSaveRender"
      :on-toggle-fullscreen="toggleFullscreen"
      :is-camera-locked="isCameraLocked"
      :is-render-paused="isRenderPaused"
      :is-saving-render="isSavingRender"
      v-on:camera-lock="toggleCameraLock()"
      v-on:render-pause="toggleRenderPause()"
    />
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene, WebDevice } from "equinox";
import StatusBar from "@/components/StatusBar.vue";
import Toolbar from "@/components/Toolbar.vue";
import CanvasOverlay from "@/components/CanvasOverlay.vue";
import Zip from "jszip";
import FileSaver from "file-saver";
import {
  getWebGlVendor,
  getWebGlRenderer,
  WebGlTimeElapsedQuery
} from "../helpers/webgl_info";
import MovingWindowEstimator from "../helpers/minimum_window";

@Component({
  components: {
    CanvasOverlay,
    StatusBar,
    Toolbar
  }
})
export default class extends Vue {
  @Prop() private equinox!: typeof import("equinox");

  @Prop() private scene!: WebScene;

  @Prop() private assetsInFlight!: number;

  @Prop() private loadAssets!: (assets: string[], compression: string) => void;
  @Prop() private clearAssets!: () => void;
  @Prop() private getAsset!: (assets: string) => Uint8Array | null;

  private isExpensiveUpdate: boolean = false;
  private updateError: string | null = null;

  private device!: WebDevice;

  mounted() {
    const canvas = this.$refs.canvas as HTMLCanvasElement;

    this.canvas = canvas;

    this.context = canvas.getContext("webgl2", {
      alpha: false,
      antialias: false,
      depth: false,
      premultipliedAlpha: false,
      stencil: false,
      powerPreference: "high-performance",
      preserveDrawingBuffer: false
    });

    this.extension = this.context!.getExtension("WEBGL_lose_context");

    if (this.context === null) {
      alert("Sorry, your browser does not appear to support WebGL2!");
    }

    this.gpuTimeQueries = new WebGlTimeElapsedQuery(this.context!);

    this.canvas.addEventListener("webglcontextlost", event => {
      this.gpuTimeQueries!.clear();
      this.clearAssets();
      this.device.context_lost();
      event.preventDefault();
    });

    this.device = new this.equinox.WebDevice(this.context!);
    this.$emit("device-created", this.device);

    if (this.device.texture_compression() === "None") {
      alert(
        "Your device or browser does not support compressed textures, some features may not work!"
      );
    }

    this.canvas.focus();

    this.animationFrame = requestAnimationFrame(this.renderLoop);

    this.$root.$on("save-scene-request", () => {
      this.isSceneSaveRequested = true;
    });
  }

  private animationFrame: number | null = null;
  private showStatusBar: boolean = true;

  private canvasStyle: string = "";

  private resizeAndMaintainAspectRatio() {
    const clientW = this.$el.clientWidth;
    const clientH = this.$el.clientHeight;

    if (clientW === 0 || clientH === 0) {
      return; // spurious resize event
    }

    const canvasW = this.canvas!.width;
    const canvasH = this.canvas!.height;

    const ratioX = canvasW / clientW;
    const ratioY = canvasH / clientH;

    if (ratioX < ratioY) {
      this.canvasStyle = `
        width: ${Math.max(1, canvasW / ratioY)}px;
        transform: translateX(-50%); left: 50%;

        height: 100%;
      `;
    } else {
      this.canvasStyle = `
        height: ${Math.max(1, canvasH / ratioX)}px;
        transform: translateY(-50%); top: 50%;

        width:  100%;
      `;
    }
  }

  private keys: { [x: string]: boolean } = {};
  private theta: number = Math.PI / 2;
  private phi: number = -Math.PI / 2;
  private thetaChange: number = 0;
  private phiChange: number = 0;
  private movementSpeed: number = 0.1;
  private mouseMoved: boolean = false;
  private thetaEstimator = new MovingWindowEstimator(10);
  private phiEstimator = new MovingWindowEstimator(10);

  private captured: boolean = false;

  private cpuFrameTimeEstimator = new MovingWindowEstimator(30);
  private gpuFrameTimeEstimator = new MovingWindowEstimator(30);
  private syncIntervalEstimator = new MovingWindowEstimator(30);

  private cpuFrameTime: number | null = null;
  private gpuFrameTime: number | null = null;
  private syncInterval: number | null = null;

  private gpuTimeQueries: WebGlTimeElapsedQuery | null = null;

  private extension: WEBGL_lose_context | null = null;

  private isContextLost: boolean = false;

  private mustSaveRender: boolean = false;
  private isSavingRender: boolean = false;
  private screenshot: Blob | null = null;
  private isSceneSaveRequested: boolean = false;
  private isCameraLocked: boolean = false;
  private isRenderPaused: boolean = false;

  private toggleFullscreen() {
    if (document.fullscreenElement === null) {
      this.$el.requestFullscreen();
    } else {
      document.exitFullscreen();
    }
  }

  private toggleCameraLock() {
    this.isCameraLocked = !this.isCameraLocked;
  }

  private toggleRenderPause() {
    this.isRenderPaused = !this.isRenderPaused;
  }

  private onSaveRender() {
    this.mustSaveRender = true;
  }

  private loseContext() {
    if (this.extension !== null) {
      this.extension.loseContext();
    }
  }

  private restoreContext() {
    if (this.extension !== null) {
      this.extension.restoreContext();
    }
  }

  private pressKey(key: string) {
    if (!this.captured) {
      return;
    }

    this.keys[key] = true;
  }

  private saveScreenshot() {
    this.mustSaveRender = true;
  }

  private releaseKey(key: string) {
    delete this.keys[key];
  }

  private moveCamera(event: MouseEvent) {
    if (!this.captured) {
      return;
    }

    if (event.movementX === 0 && event.movementY === 0) {
      return;
    }

    if (!this.mouseMoved) {
      // reconstruct coordinates from scene json in case they changed

      const direction = this.scene.json().camera.direction;

      this.phi = Math.atan2(direction[2], direction[0]);
      this.theta = Math.acos(direction[1]);
    }

    this.phiChange += -event.movementX * 0.001;
    this.thetaChange += event.movementY * 0.001;

    this.mouseMoved = true;
  }

  private enterCapture() {
    if (this.canvas !== null) {
      this.canvas.requestPointerLock();
      this.captured = true;
    }
  }

  private leaveCapture() {
    if (this.canvas !== null) {
      document.exitPointerLock();
      this.captured = false;
      this.keys = {};
    }
  }

  private onKeyPress(event: KeyboardEvent) {
    if (event.shiftKey && event.key === "K" && this.extension !== null) {
      this.extension.loseContext();
    }

    if (event.shiftKey && event.key === "L" && this.extension !== null) {
      this.extension.restoreContext();
    }

    if (event.shiftKey && event.key === "O") {
      this.showStatusBar = !this.showStatusBar;
    }

    // ...
  }

  private onMouseWheel(amount: number) {
    this.movementSpeed *= Math.pow(1.1, amount / 64);
  }

  get contextVendor(): string {
    return this.context === null ? "unknown" : getWebGlVendor(this.context!);
  }

  get contextRenderer(): string {
    return this.context === null ? "unknown" : getWebGlRenderer(this.context!);
  }

  private canvas: HTMLCanvasElement | null = null;
  private context: WebGL2RenderingContext | null = null;
  private canvasWidth: number = 0;
  private canvasHeight: number = 0;
  private sppmPhotons: number = 0;
  private sppmPasses: number = 0;

  private lastVsync: number = 0;

  private async performDeviceUpdate() {
    if (this.isContextLost) {
      return; // not ready
    }

    // TODO: it would be nice to not have to call this each time since it scans the
    // entire scene contents but it's not that slow; optimize only when we need to.

    await this.loadAssets(
      this.scene.assets(),
      this.device.texture_compression()
    );

    this.device.update(this.scene, this.getAsset);
  }

  async renderLoop() {
    const start = performance.now();

    if (this.lastVsync !== 0) {
      this.syncIntervalEstimator.addSample(start - this.lastVsync);
    }

    this.lastVsync = start;

    if (
      this.canvas !== null &&
      this.context !== null &&
      this.canvas.clientWidth != 0 &&
      this.canvas.clientHeight != 0
    ) {
      this.isContextLost = this.context.isContextLost();

      let forward = 0;
      let sideways = 0;
      let upwards = 0;

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

      if (this.keys["q"]) {
        upwards += 1.0;
      }

      if (this.keys["z"]) {
        upwards -= 1.0;
      }

      if (forward != 0 || upwards != 0 || sideways != 0) {
        this.scene.move_camera(
          sideways * this.movementSpeed,
          upwards * this.movementSpeed,
          forward * this.movementSpeed
        );
      }

      this.thetaEstimator.addSample(this.thetaChange);
      this.phiEstimator.addSample(this.phiChange);
      this.thetaChange = 0;
      this.phiChange = 0;

      if (this.mouseMoved) {
        this.theta += this.thetaEstimator.average()!;
        this.phi += this.phiEstimator.average()!;

        if (this.theta > Math.PI - 0.01) {
          this.theta = Math.PI - 0.01;
        }

        if (this.theta < 0.01) {
          this.theta = 0.01;
        }

        let x = Math.sin(this.theta) * Math.cos(this.phi);
        let z = Math.sin(this.theta) * Math.sin(this.phi);
        let y = Math.cos(this.theta);

        this.scene.set_camera_direction(x, y, z);

        this.mouseMoved = false;
      }

      this.sppmPhotons = this.device.sppm_photons();
      this.sppmPasses = this.device.sppm_passes();

      try {
        if (!this.isRenderPaused) {
          if (this.isExpensiveUpdate) {
            await this.performDeviceUpdate();
            this.isExpensiveUpdate = false;
          } else if (this.device.is_expensive_update(this.scene)) {
            this.isExpensiveUpdate = true;
          } else {
            await this.performDeviceUpdate();
          }

          this.updateError = null;
        }

        if (this.isRenderPaused || !this.isExpensiveUpdate) {
          this.canvas.width = this.scene.raster_width();
          this.canvas.height = this.scene.raster_height();
          this.resizeAndMaintainAspectRatio();
          this.canvasWidth = this.canvas.width;
          this.canvasHeight = this.canvas.height;
        }

        const refineTime = this.gpuTimeQueries!.timeElapsed(() => {
          if (!this.isRenderPaused) {
            this.device.refine();
          }
        });

        this.device.present();

        if (this.mustSaveRender) {
          this.generateScreenshotZip();
          this.mustSaveRender = false;
        }

        if (this.isSceneSaveRequested) {
          this.performSceneSave();
          this.isSceneSaveRequested = false;
        }

        this.gpuFrameTimeEstimator.addSample(refineTime);
      } catch (e) {
        this.updateError = e.message;
        this.isRenderPaused = true;
      }
    }

    const time = (performance.now() - start) / 1000;

    this.cpuFrameTimeEstimator.addSample(time);

    this.cpuFrameTime = this.cpuFrameTimeEstimator.minimum();
    this.gpuFrameTime = this.gpuFrameTimeEstimator.minimum();
    this.syncInterval = this.syncIntervalEstimator.average();

    this.animationFrame = requestAnimationFrame(this.renderLoop);
  }

  private async generateScreenshotZip() {
    this.isSavingRender = true;

    try {
      const zip = new Zip();

      const render = new Promise<Blob>(resolve => {
        this.canvas!.toBlob(blob => resolve(blob!));
      });

      const info = {
        sppm_passes: this.device.sppm_passes(),
        sppm_photons: this.device.sppm_photons(),
        vendor: this.contextVendor,
        renderer: this.contextRenderer,
        version: this.equinox.version()
      };

      zip.file("scene.json", JSON.stringify(this.scene.json(), null, 2));
      zip.file("meta.json", JSON.stringify(info, null, 2));
      zip.file("render.png", await render);

      FileSaver.saveAs(await zip.generateAsync({ type: "blob" }), "render.zip");
    } finally {
      this.isSavingRender = false;
    }
  }

  private async performSceneSave() {
    const json = this.scene.json();
    const name = this.scene.name();

    const thumbnailWidth = 320;
    const thumbnailHeight = 180;

    const [w, h] = this.resizeToRatio(
      this.canvas!.width,
      this.canvas!.height,
      thumbnailWidth,
      thumbnailHeight
    );

    this.$root.$emit(
      "save-scene-response",
      name,
      json,
      this.createThumbnail(this.canvas!, w, h)
    );
  }

  private resizeToRatio(
    srcW: number,
    srcH: number,
    maxW: number,
    maxH: number
  ): [number, number] {
    const ratioX = srcW / maxW;
    const ratioY = srcH / maxH;

    if (ratioX < ratioY) {
      return [srcW / ratioY, maxH];
    } else {
      return [maxW, srcH / ratioX];
    }
  }

  private createThumbnail(
    src: HTMLCanvasElement,
    dstW: number,
    dstH: number
  ): string {
    let dst = document.createElement("canvas");
    let srcW = src.width;
    let srcH = src.height;
    dst.width = srcW;
    dst.height = srcH;

    dst.getContext("2d")!.drawImage(src, 0, 0);

    while (srcW / 2 >= dstW && srcH / 2 >= dstH) {
      const tmp = document.createElement("canvas");
      tmp.width = srcW / 2;
      tmp.height = srcH / 2;

      tmp
        .getContext("2d")!
        .drawImage(dst, 0, 0, dst.width, dst.height, 0, 0, srcW / 2, srcH / 2);
      dst = tmp;

      srcW /= 2;
      srcH /= 2;
    }

    if (dstW != srcW && dstH != srcH) {
      const final = document.createElement("canvas");
      final.width = dstW;
      final.height = dstH;

      final
        .getContext("2d")!
        .drawImage(dst, 0, 0, dst.width, dst.height, 0, 0, dstW, dstH);
      dst = final;
    }

    return dst.toDataURL("image/jpeg", 0.9);
  }
}
</script>

<style scoped>
.container {
  position: relative;
  width: 100%;
  height: 100%;
  user-select: none;
}

.canvas {
  position: absolute;
  bottom: 0;
  right: 0;
  margin: 0;
  outline: none;
}

.canvas-locked {
  pointer-events: none;
}

.status-bar {
  position: absolute;
  bottom: 0;
  width: 100%;
  height: 18px;
  border-top: 1px solid #777777;
  z-index: 1;
}
</style>
