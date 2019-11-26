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
          <AdvancedEditor :scene="scene" :load-assets="loadAssets" />
        </template>
        <template slot="tab-head-environment">Environment</template>
        <template slot="tab-panel-environment">
          <EnvironmentEditor :scene="scene" :load-assets="loadAssets" />
        </template>
        <template slot="tab-head-documentation">Documentation</template>
        <template slot="tab-panel-documentation">
          <DocumentationEditor />
        </template>
        <template slot="tab-head-save-load">Save/Load</template>
        <template slot="tab-panel-save-load">
          <SaveLoadEditor :scene="scene" :load-assets="loadAssets" />
        </template>
      </EditorContainer>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import LoadingOverlay from "@/components/LoadingOverlay.vue";
import EnvironmentEditor from "@/components/EnvironmentEditor.vue";
import EditorContainer from "@/components/EditorContainer.vue";
import DocumentationEditor from "@/components/DocumentationEditor.vue";
import AdvancedEditor from "@/components/AdvancedEditor.vue";
import { WebScene, WebDevice } from "equinox";
import localforage from "localforage";
import pako from "pako";
import MovingWindowEstimator from "./helpers/minimum_window";
import CanvasContainer from "@/components/CanvasContainer.vue";
import SaveLoadEditor from "@/components/SaveLoadEditor.vue";

@Component({
  components: {
    LoadingOverlay,
    EnvironmentEditor,
    EditorContainer,
    DocumentationEditor,
    CanvasContainer,
    AdvancedEditor,
    SaveLoadEditor
  }
})
export default class App extends Vue {
  @Prop() private equinox!: typeof import("equinox");

  private scene = new this.equinox.WebScene();

  private editorTabsAbove = ["environment"];
  private editorTabsBelow = ["documentation", "save-load", "advanced"];
  private defaultEditorTab = "documentation";

  private loadingCount: number = 0;
  private downloadingCount: number = 0;

  async loadAssets(assets: string[]): Promise<void> {
    const sceneAssets = this.scene.assets() as string[];

    assets = assets.filter(asset => {
      // asset is already loaded on scene
      return !sceneAssets.includes(asset);
    });

    const promises = [];

    for (const asset of assets) {
      promises.push(this.fetchAsset(asset));
    }

    for (const [index, buffer] of (await Promise.all(promises)).entries()) {
      this.scene.insert_asset(assets[index], new Uint8Array(buffer));
    }
  }

  async fetchAsset(url: string): Promise<ArrayBuffer> {
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

  mounted() {
    // TODO: load a default asset-less prefab from inside JS
    this.scene.set_default_scene();
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
