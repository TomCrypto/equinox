<template>
  <div id="app">
    <div class="canvas-panel">
      <CanvasContainer :equinox="equinox" :scene="scene" />

      <LoadingOverlay :loading-count="loadingCount" :downloading-count="downloadingCount" />
    </div>
    <div class="editor-panel">
      <EditorContainer
        class="editor"
        :tabs-above="editorTabsAbove"
        :tabs-below="editorTabsBelow"
        :defaultTab="defaultEditorTab"
      >
        <template slot="tab-head-advanced">Advanced Editor</template>
        <template slot="tab-panel-advanced">
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
        <template slot="tab-head-environment">Environment</template>
        <template slot="tab-panel-environment">
          <EnvironmentEditor :scene="scene" :load-asset="load_asset" />
        </template>
        <template slot="tab-head-documentation">Documentation</template>
        <template slot="tab-panel-documentation">
          <DocumentationEditor />
        </template>
      </EditorContainer>
    </div>

    <!--<JsonEditor
      v-if="isEditingJson"
      :payload="sceneJson()"
      :on-update-scene="updateScene"
      :on-close="closeEditor"
    />

    <Toolbar
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

    <!--
    <DownloadOverlay
      v-if="screenshot !== null"
      :render="screenshot"
      :on-close="downloadOverlayClosed"
    />
    -->
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
import DocumentationEditor from "@/components/DocumentationEditor.vue";
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
import CanvasContainer from "@/components/CanvasContainer.vue";

@Component({
  components: {
    StatusBar,
    Toolbar,
    LoadingOverlay,
    JsonEditor,
    DownloadOverlay,
    EnvironmentEditor,
    EditorContainer,
    DocumentationEditor,
    CanvasContainer
  }
})
export default class App extends Vue {
  @Prop() private equinox!: typeof import("equinox");

  private scene!: WebScene;
  private device!: WebDevice;

  private editorTabsAbove = ["environment"];
  private editorTabsBelow = ["documentation", "advanced"];
  private defaultEditorTab = "documentation";

  private loadingCount: number = 0;
  private downloadingCount: number = 0;

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

  created() {
    this.scene = new this.equinox.WebScene();
    this.scene.set_default_scene();

    /*const asset = "assets/old_outdoor_theater_4k.raw";

    (async () => {
      await this.load_asset(asset);
      this.scene.set_envmap(asset);
    })();*/
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
  float: left;

  width: calc(50% - 2px);
  height: 100vh;

  position: relative;
  border: 0;
  border-right: 2px;
  border-style: solid;
  border-color: #5a5a5a;
  margin: 0;
  outline: none;
  padding: 0px;
  background-color: black;
}

.canvas-panel:fullscreen {
  position: absolute;
  width: 100vw;
  height: 100vh;
  border: 0;
}

.editor-panel {
  float: right;
  width: 50%;
}

.editor {
  height: 100vh;
}
</style>
