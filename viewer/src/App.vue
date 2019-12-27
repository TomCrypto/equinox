<template>
  <div id="app">
    <div class="canvas-panel">
      <CanvasContainer
        :equinox="equinox"
        :scene="scene"
        :assets-in-flight="assetsInFlight"
        :load-assets="loadAssets"
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
        <template slot="tab-head-advanced"
          >Advanced Editor</template
        >
        <template slot="tab-panel-advanced">
          <AdvancedEditor :scene="scene" />
        </template>
        <template slot="tab-head-environment"
          >Environment</template
        >
        <template slot="tab-panel-environment">
          <EnvironmentEditor :scene="scene" />
        </template>
        <template slot="tab-head-documentation"
          >Documentation</template
        >
        <template slot="tab-panel-documentation">
          <DocumentationEditor />
        </template>
        <template slot="tab-head-save-load"
          >Save/Load</template
        >
        <template slot="tab-panel-save-load">
          <SaveLoadEditor :scene="scene" />
        </template>
        <template slot="tab-head-licensing"
          >Licensing</template
        >
        <template slot="tab-panel-licensing">
          <LicensingEditor
            :licensing="equinox.licensing()"
            :version="equinox.version()"
          />
        </template>
      </EditorContainer>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import EnvironmentEditor from "@/components/EnvironmentEditor.vue";
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

  private editorTabsAbove = ["environment"];
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

  async loadAssets(assets: string[], compression: string): Promise<void> {
    for (const asset of this.assets.keys()) {
      if (!assets.includes(asset)) {
        this.assets.delete(asset);
      }
    }

    assets = assets.filter(asset => !this.assets.has(asset));

    const promises = [];

    for (const asset of assets) {
      const url = this.url_for_asset(asset, compression);

      const promise = this.assetDownloads.get(asset) || this.fetchAsset(url);

      promises.push(promise);
      this.assetDownloads.set(asset, promise);
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
        const buffer = await (await fetch(new Request(url))).arrayBuffer();
        data = new Blob([pako.inflate(new Uint8Array(buffer)).buffer]);

        await this.store.setItem(url, data);
      }

      return new Response(data).arrayBuffer();
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

    throw new Error("unknown texture compression format reported");
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
  user-select: none;
  background-color: #1a1a1a;
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
