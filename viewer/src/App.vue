<template>
  <div id="app">
    <canvas
      ref="canvas"
      tabindex="0"
      v-on:mousedown="enterCapture()"
      v-on:mouseup="leaveCapture()"
      v-on:mouseleave="leaveCapture()"
      v-on:mousemove="moveCamera($event)"
      v-on:keydown="pressKey($event.key)"
      v-on:keyup="releaseKey($event.key)"
      v-on:keypress="onKeyPress($event)"
      v-on:contextmenu="$event.preventDefault()"
    />

    <JsonEditor
      v-if="isEditingJson"
      :payload="sceneJson()"
      :on-update-scene="updateScene"
      :on-close="closeEditor"
    />

    <Toolbar :on-save-screenshot="saveScreenshot" :on-edit-json="editJson" />

    <StatusBar
      v-if="canvas !== null"
      :sample-count="sampleCount"
      :is-context-lost="isContextLost"
      :width="canvasWidth"
      :height="canvasHeight"
      :vendor="contextVendor"
      :renderer="contextRenderer"
      :cpuFrameTime="cpuFrameTime"
      :gpuFrameTime="gpuFrameTime"
      :syncInterval="syncInterval"
    />

    <LoadingOverlay
      v-if="canvas !== null"
      :loading-count="loadingCount"
      :downloading-count="downloadingCount"
    />

    <DownloadOverlay
      v-if="screenshot !== null"
      :render="screenshot"
      :on-close="downloadOverlayClosed"
    />
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import StatusBar from "@/components/StatusBar.vue";
import LoadingOverlay from "@/components/LoadingOverlay.vue";
import Toolbar from "@/components/Toolbar.vue";
import JsonEditor from "@/components/JsonEditor.vue";
import DownloadOverlay from "@/components/DownloadOverlay.vue";
import { WebScene, WebDevice } from "equinox";
import localforage from "localforage";
import Zip from "jszip";
import pako from "pako";
import {
  getWebGlVendor,
  getWebGlRenderer,
  WebGlTimeElapsedQuery
} from "./helpers/webgl_info";
import MovingWindowEstimator from "./helpers/minimum_window";

@Component({
  components: {
    StatusBar,
    Toolbar,
    LoadingOverlay,
    JsonEditor,
    DownloadOverlay
  }
})
export default class App extends Vue {
  @Prop() private equinox!: typeof import("equinox");

  private scene!: WebScene;
  private device!: WebDevice;

  private keys: { [x: string]: boolean } = {};
  private theta: number = Math.PI / 2;
  private phi: number = Math.PI / 2;
  private mouseMoved: boolean = false;

  private captured: boolean = false;

  private cpuFrameTimeEstimator = new MovingWindowEstimator(30);
  private gpuFrameTimeEstimator = new MovingWindowEstimator(30);
  private syncIntervalEstimator = new MovingWindowEstimator(30);

  private cpuFrameTime: number | null = null;
  private gpuFrameTime: number | null = null;
  private syncInterval: number | null = null;

  private gpuTimeQueries: WebGlTimeElapsedQuery | null = null;

  private loadingCount: number = 0;
  private downloadingCount: number = 0;

  private isEditingJson: boolean = false;

  private extension: WEBGL_lose_context | null = null;

  private isContextLost: boolean = false;

  private mustSaveScreenshot: boolean = false;
  private screenshot: Blob | null = null;

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

  private downloadOverlayClosed() {
    this.screenshot = null;
  }

  private editJson() {
    this.isEditingJson = !this.isEditingJson;
  }

  private closeEditor() {
    this.isEditingJson = false;
  }

  private sceneJson(): object {
    return {
      json: this.scene.json(),
      assets: this.scene.assets()
    };
  }

  private saveScreenshot() {
    this.mustSaveScreenshot = true;
  }

  private releaseKey(key: string) {
    delete this.keys[key];
  }

  private moveCamera(event: MouseEvent) {
    if (!this.captured) {
      return;
    }

    this.phi += -event.movementX * 0.001;
    this.theta += event.movementY * 0.001;

    if (this.theta > Math.PI - 0.01) {
      this.theta = Math.PI - 0.01;
    }

    if (this.theta < 0.01) {
      this.theta = 0.01;
    }

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

  private async updateScene(json: object, assets: string[]): Promise<boolean> {
    const oldAssets = this.scene.assets();
    const promises = [];

    for (const asset of assets) {
      if (oldAssets.includes(asset)) {
        continue;
      }

      promises.push(this.load_asset(asset));
    }

    await Promise.all(promises);

    for (const asset of oldAssets) {
      if (!assets.includes(asset)) {
        this.scene.remove_asset(asset);
      }
    }

    try {
      this.scene.set_json(json);
      return true;
    } catch {
      return false;
    }
  }

  private onKeyPress(event: KeyboardEvent) {
    if (event.shiftKey && event.key === "K" && this.extension !== null) {
      this.extension.loseContext();
    }

    if (event.shiftKey && event.key === "L" && this.extension !== null) {
      this.extension.restoreContext();
    }

    // ...
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
  private sampleCount: number = 0;

  created() {
    this.scene = new this.equinox.WebScene();
    this.scene.setup_test_scene();

    const asset = "assets/blue_grotto_4k.raw";

    (async () => {
      await this.load_asset(asset);
      this.scene.set_envmap(asset);
    })();
  }

  private animationFrame: number | null = null;

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
      this.device.context_lost();
      event.preventDefault();
    });

    this.device = new this.equinox.WebDevice(this.context!);

    this.canvas.focus();

    this.animationFrame = requestAnimationFrame(this.renderLoop);
  }

  private lastVsync: number = 0;

  renderLoop() {
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
        this.scene.move_camera(sideways * 0.1, 0, forward * 0.1);
      }

      if (this.mouseMoved) {
        let x = Math.sin(this.theta) * Math.cos(this.phi);
        let z = Math.sin(this.theta) * Math.sin(this.phi);
        let y = Math.cos(this.theta);

        this.scene.set_camera_direction(x, y, z);

        this.mouseMoved = false;
      }

      this.canvas.width = this.canvas.clientWidth;
      this.canvas.height = this.canvas.clientHeight;
      this.canvasWidth = this.canvas.width;
      this.canvasHeight = this.canvas.height;

      this.sampleCount = this.device.sample_count();

      this.scene.set_raster_dimensions(this.canvas.width, this.canvas.height);

      this.device.update(this.scene);

      const refineTime = this.gpuTimeQueries!.timeElapsed(() => {
        this.device.refine();
        this.device.render();
      });

      this.gpuFrameTimeEstimator.addSample(refineTime);

      if (this.mustSaveScreenshot) {
        this.generateScreenshotZip();
      }
    }

    const time = (performance.now() - start) / 1000;

    this.cpuFrameTimeEstimator.addSample(time);

    this.cpuFrameTime = this.cpuFrameTimeEstimator.minimum();
    this.gpuFrameTime = this.gpuFrameTimeEstimator.minimum();
    this.syncInterval = this.syncIntervalEstimator.average();

    this.animationFrame = requestAnimationFrame(this.renderLoop);
    this.mustSaveScreenshot = false; // avoid spurious screenshot
  }

  private async generateScreenshotZip() {
    const zip = new Zip();

    const render = new Promise<Blob>(resolve => {
      this.canvas!.toBlob(blob => resolve(blob!));
    });

    const info = {
      samples: this.device.sample_count(),
      vendor: this.contextVendor,
      renderer: this.contextRenderer,
      version: this.equinox.version()
    };

    zip.file("scene.json", JSON.stringify(this.sceneJson(), null, 2));
    zip.file("meta.json", JSON.stringify(info, null, 2));
    zip.file("render.png", await render);

    this.screenshot = await zip.generateAsync({ type: "blob" });
  }

  async load_asset(url: string) {
    const data = await this.fetch_asset_data(url);
    this.scene.insert_asset(url, new Uint8Array(data));
  }

  async fetch_asset_data(url: string): Promise<ArrayBuffer> {
    this.loadingCount += 1;

    try {
      let data = (await localforage.getItem(url)) as Blob | null;

      if (data === null) {
        this.downloadingCount += 1;

        try {
          const buffer = await (await fetch(new Request(url))).arrayBuffer();
          data = new Blob([pako.inflate(new Uint8Array(buffer)).buffer]);

          await localforage.setItem(url, data);
        } finally {
          this.downloadingCount -= 1;
        }
      }

      return new Response(data).arrayBuffer();
    } finally {
      this.loadingCount -= 1;
    }
  }
}
</script>

<style>
body {
  margin: 0;
}

.status {
  position: absolute;
  bottom: 0;
  width: 100%;
  height: 18px !important;
  border-top: 1px solid #777777;
  z-index: 1;
}

canvas {
  width: 100vw;
  height: 100vh;
  background-color: black;
  display: block;
}
</style>
