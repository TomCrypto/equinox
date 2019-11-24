<template>
  <div id="app">
    <div class="canvas-panel">
      <div class="canvas-container">
        <!-- canvas placeholder -->
        <canvas
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
      </div>

      <div class="changelog">sup</div>
    </div>
    <div class="editor-panel">
      <EditorContainer
        class="editor"
        :tabs-above="editorTabsAbove"
        :tabs-below="editorTabsBelow"
        :defaultTab="defaultEditorTab"
      >
        <template slot="tab-head-test">Advanced Editor</template>
        <template slot="tab-panel-test">
          <p>
            Obtain full control by directly editing the scene's underlying representation. Note that
            some changes (especially changing the geometry modifier stack and changing non-symbolic
            parameters) may trigger shader rebuilds which can take a few seconds.
          </p>
          <p>
            On Windows, shader builds can be very slow due to the ANGLE GLSL to HLSL conversion. It
            is recommended to switch to native OpenGL if possible.
          </p>
          <hr />
          <p>(code editor here)</p>
        </template>
        <template slot="tab-head-blah">Environment</template>
        <template slot="tab-panel-blah"></template>
      </EditorContainer>
    </div>
    <!--<canvas
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
    />-->

    <JsonEditor
      v-if="isEditingJson"
      :payload="sceneJson()"
      :on-update-scene="updateScene"
      :on-close="closeEditor"
    />

    <!--<Toolbar
      :on-save-screenshot="saveScreenshot"
      :on-edit-json="editJson"
      :on-edit-environment="editEnvironment"
    />-->

    <!--
    <StatusBar
      v-if="canvas !== null"
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
    -->

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

    <EnvironmentEditor v-if="showEnvironmentEditor" :scene="scene" :load-asset="load_asset" />
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import StatusBar from "@/components/StatusBar.vue";
import LoadingOverlay from "@/components/LoadingOverlay.vue";
import Toolbar from "@/components/Toolbar.vue";
import JsonEditor from "@/components/JsonEditor.vue";
import DownloadOverlay from "@/components/DownloadOverlay.vue";
import EnvironmentEditor from "@/components/EnvironmentEditor.vue";
import EditorContainer from "@/components/EditorContainer.vue";
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
    DownloadOverlay,
    EnvironmentEditor,
    EditorContainer
  }
})
export default class App extends Vue {
  @Prop() private equinox!: typeof import("equinox");

  private scene!: WebScene;
  private device!: WebDevice;

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

  private loadingCount: number = 0;
  private downloadingCount: number = 0;

  private isEditingJson: boolean = false;

  private editorTabsAbove = ["test"];
  private editorTabsBelow = ["blah"];
  private defaultEditorTab = "test";

  private extension: WEBGL_lose_context | null = null;
  private showEnvironmentEditor: boolean = false;

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

  private editEnvironment() {
    this.showEnvironmentEditor = !this.showEnvironmentEditor;
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

    if (event.movementX === 0 && event.movementY === 0) {
      return;
    }

    if (!this.mouseMoved) {
      // reconstruct coordinates from scene json in case they changed

      const direction = this.scene.json().camera.direction;

      this.phi = Math.atan2(direction.z, direction.x);
      this.theta = Math.acos(direction.y);
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

  created() {
    this.scene = new this.equinox.WebScene();
    this.scene.set_default_scene();

    /*const asset = "assets/old_outdoor_theater_4k.raw";

    (async () => {
      await this.load_asset(asset);
      this.scene.set_envmap(asset);
    })();*/
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

      /*this.canvas.width = this.canvas.clientWidth;
      this.canvas.height = this.canvas.clientHeight;*/
      this.canvasWidth = this.canvas.width;
      this.canvasHeight = this.canvas.height;

      this.sppmPhotons = this.device.sppm_photons();
      this.sppmPasses = this.device.sppm_passes();

      this.scene.set_raster_dimensions(this.canvas.width, this.canvas.height);

      try {
        this.device.update(this.scene);

        const refineTime = this.gpuTimeQueries!.timeElapsed(() => {
          this.device.refine();
          this.device.render();
        });

        this.gpuFrameTimeEstimator.addSample(refineTime);

        if (this.mustSaveScreenshot) {
          this.generateScreenshotZip();
        }
      } catch (e) {
        console.error(e);
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
      sppm_passes: this.device.sppm_passes(),
      sppm_photons: this.device.sppm_photons(),
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
  overflow: hidden;
  font-family: "Trebuchet MS", "Lucida Sans Unicode", "Lucida Grande",
    "Lucida Sans", Arial, sans-serif;
  user-select: none;
  background-color: #1a1a1a;
}

.status {
  position: absolute;
  bottom: 0;
  width: 100%;
  height: 18px !important;
  border-top: 1px solid #777777;
  z-index: 1;
}

.canvas-panel {
  display: flex;
  flex-direction: column;
  justify-content: center;
  float: left;
  width: 50%;
  height: 100vh;
  flex: 1;
}

.canvas-container {
  padding: 0px;
  background-color: black;
  width: calc(100% - 8x);
  height: calc(50% - 8px);
  position: relative;
  border: 4px solid #1a1a1a;
  border-radius: 12px;
  margin: 0;
  outline: none;
}

.changelog {
  flex: 1;
}

.editor-panel {
  float: right;
  width: 50%;
}

.editor {
  height: 100vh;
}

canvas {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  right: 0;
  transform: translateX(-50%);
  width: 50%;
  height: 100%;
  margin: 0;
  outline: none;
}
</style>
