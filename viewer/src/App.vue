<template>
  <div id="app">
    <div class="canvas-panel">
      <CanvasContainer
        :equinox="equinox"
        :scene="scene"
        :assets-in-flight="assetsInFlight"
        :load-assets="loadAssets"
        :clear-assets="clearAssets"
        :get-asset="getAsset"
      />
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
          <AdvancedEditor :scene="scene" />
        </template>
        <template slot="tab-head-camera">Camera</template>
        <template slot="tab-panel-camera">
          <CameraEditor :scene="scene" />
        </template>
        <template slot="tab-head-environment">Environment</template>
        <template slot="tab-panel-environment">
          <EnvironmentEditor :scene="scene" />
        </template>
        <template slot="tab-head-documentation">Documentation</template>
        <template slot="tab-panel-documentation">
          <DocumentationEditor />
        </template>
        <template slot="tab-head-save-load">Save/Load</template>
        <template slot="tab-panel-save-load">
          <SaveLoadEditor :scene="scene" />
        </template>
        <template slot="tab-head-licensing">Licensing</template>
        <template slot="tab-panel-licensing">
          <LicensingEditor :licensing="equinox.licensing()" :version="equinox.version()" />
        </template>
      </EditorContainer>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import EnvironmentEditor from "@/components/EnvironmentEditor.vue";
import CameraEditor from "@/components/CameraEditor.vue";
import EditorContainer from "@/components/EditorContainer.vue";
import DocumentationEditor from "@/components/DocumentationEditor.vue";
import LicensingEditor from "@/components/LicensingEditor.vue";
import AdvancedEditor from "@/components/AdvancedEditor.vue";
import { WebScene, WebDevice } from "equinox";
import localforage from "localforage";
import pako from "pako";
import MovingWindowEstimator from "./helpers/minimum_window";
import CanvasContainer from "@/components/CanvasContainer.vue";
import SaveLoadEditor from "@/components/SaveLoadEditor.vue";
import DefaultScene from "./prefab/DefaultScene";

@Component({
  components: {
    EnvironmentEditor,
    CameraEditor,
    EditorContainer,
    DocumentationEditor,
    LicensingEditor,
    CanvasContainer,
    AdvancedEditor,
    SaveLoadEditor
  }
})
export default class App extends Vue {
  @Prop() private equinox!: typeof import("equinox");

  private scene = new this.equinox.WebScene();

  private editorTabsAbove = ["camera", "environment"];
  private editorTabsBelow = [
    "documentation",
    "save-load",
    "advanced",
    "licensing"
  ];
  private defaultEditorTab = "documentation";

  private assetDownloads = new Map<string, Promise<ArrayBuffer>>();
  private assetsInFlight = 0;

  private assets: Map<string, Uint8Array> = new Map();

  private readonly store = localforage.createInstance({
    driver: localforage.INDEXEDDB,
    name: "equinox-asset-data-v2"
  });

  getAsset(asset: string): Uint8Array | null {
    return this.assets.get(asset) || null;
  }

  clearAssets() {
    this.assets.clear();
  }

  async loadAssets(assets: string[], compression: string) {
    for (const asset of this.assets.keys()) {
      if (!assets.includes(asset)) {
        this.assets.delete(asset);
      }
    }

    assets = assets.filter(asset => !this.assets.has(asset));

    const promises = [];

    for (const asset of assets) {
      const url = this.url_for_asset(asset, compression);

      const promise = this.assetDownloads.get(url) || this.fetchAsset(url);

      promises.push(promise);
      this.assetDownloads.set(url, promise);
    }

    this.assetsInFlight = this.assetDownloads.size;

    for (const [index, buffer] of (await Promise.all(promises)).entries()) {
      this.assets.set(assets[index], new Uint8Array(buffer));
    }
  }

  async fetchAsset(url: string): Promise<ArrayBuffer> {
    try {
      let data = (await this.store.getItem(url)) as Blob | null;

      if (data === null) {
        const response = await fetch(new Request(url));

        // In development fetching an unknown asset will actually succeed and return
        // a 200 response for some reason, so the exception handling here will fail.

        if (!response.ok) {
          // S3 will return a 403 error if the object doesn't exist
          if (response.status === 403 || response.status === 404) {
            throw new Error("asset not found");
          } else {
            throw new Error(`network error: ${response.status}`);
          }
        }

        const buffer = await response.arrayBuffer();
        data = new Blob([pako.inflate(new Uint8Array(buffer)).buffer]);

        await this.store.setItem(url, data);
      }

      return new Response(data).arrayBuffer();
    } catch (e) {
      throw new Error(`failed to fetch asset: ${e.message}`);
    } finally {
      this.assetDownloads.delete(url);
      this.assetsInFlight = this.assetDownloads.size;
    }
  }

  private url_for_asset(asset: string, texture_compression: string): string {
    if (!asset.endsWith(".tc.raw")) {
      return asset; // uncompressed
    }

    if (texture_compression == "S3TC") {
      return asset.replace(".tc.raw", ".s3tc.raw");
    }

    if (texture_compression == "ASTC") {
      return asset.replace(".tc.raw", ".astc.raw");
    }

    throw new Error("no texture compression format supported");
  }

  mounted() {
    this.scene.set_json(DefaultScene.json);
  }
}
</script>

<style>
body {
  margin: 0;
  overflow: hidden;
  font-family: "Trebuchet MS", "Lucida Sans Unicode", "Lucida Grande",
    "Lucida Sans", Arial, sans-serif;
  background-color: #1a1a1a;
  height: 100vh;
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

.editor-panel {
  float: right;
  width: 50%;
}

.editor {
  height: 100vh;
}
</style>
