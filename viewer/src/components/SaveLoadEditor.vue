<template>
  <div class="root">
    <div class="save-panel">
      <input class="name-input" v-model="name" type="text" placeholder="Scene name" />
      <button class="save-button" v-on:click="saveScene(name)">Save</button>
    </div>
    <div class="scene-list">
      <button
        class="load-scene"
        v-for="scene in displayScenes"
        :key="scene.name"
        :style="`background-image: url(${scene.thumbnail})`"
        v-on:click="loadScene(scene.name)"
      >{{ scene.name }}</button>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import localforage from "localforage";
import { WebScene } from "equinox";

export interface Metadata {
  thumbnail: string;
  json: object;
  assets: string[];
  timestamp: string;
}

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;

  @Prop() private loadAssets: (assets: string[]) => Promise<void>;

  private scenes: Map<string, Metadata> = new Map();

  private readonly store = localforage.createInstance({
    driver: localforage.LOCALSTORAGE,
    version: 1,
    name: "equinox-saved-scenes"
  });

  private name: string = "";

  get displayScenes(): unknown {
    const scenes = [];

    for (const [name, scene] of this.scenes.entries()) {
      scenes.push({
        name,
        thumbnail: scene.thumbnail,
        timestamp: scene.timestamp
      });
    }

    return scenes.sort((lhs, rhs) =>
      rhs.timestamp.localeCompare(lhs.timestamp)
    );
  }

  mounted() {
    this.updateFromStore();

    this.$root.$on(
      "save-scene-response",
      async (name, json, assets, thumbnail) => {
        await this.store.setItem(name, {
          json,
          assets,
          thumbnail,
          timestamp: new Date().toISOString()
        });

        this.updateFromStore();
      }
    );
  }

  // how to get screenshot from current scene here??
  private async saveScene(name: string) {
    this.$root.$emit("save-scene-request", name);
  }

  private async deleteScene(name: string) {
    await this.store.removeItem(name);
    this.updateFromStore();
  }

  private async loadScene(name: string) {
    const scene = this.scenes.get(name);

    if (scene === undefined) {
      throw new Error("scene did not exist");
    }

    await this.loadAssets(scene.assets);

    // TODO: error handling here in case the scene is bad...

    this.scene.set_json(scene.json);

    for (const asset of this.scene.assets()) {
      if (!scene.assets.includes(asset)) {
        this.scene.remove_asset(asset);
      }
    }
  }

  private async updateFromStore() {
    const scenes = new Map();

    for (const name of await this.store.keys()) {
      scenes.set(name, await this.store.getItem(name));
    }

    this.scenes = scenes;

    // TODO: load prefab scenes (and save them to localstorage) if they were missing
    // store the prefab scenes in some separate helper modules and put the thumbnails in public/
  }
}
</script>

<style scoped>
.root {
  height: 100%;
  display: flex;
  flex-direction: column;
  user-select: none;
}

.save-panel {
  flex: 0;
  display: flex;
  flex-direction: row;
}

.name-input {
  flex: 2;
  user-select: text;
}

.save-button {
  flex: 1;
}

.scene-list {
  flex: 1;
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
}

.load-scene {
  flex: 1;
  background-color: black;
  background-position: center;
  background-repeat: no-repeat;
  background-size: contain;
  min-width: 320px;
  min-height: 180px;
  text-decoration: none;
  margin: 3px;
  border-radius: 6px;
  box-shadow: inset 0px 0px 5px 0px #555555;
  border: 1px solid #555555;
  cursor: pointer;
  padding-top: 4px;
}

.load-scene:active {
  position: relative;
  top: 2px;
}
</style>
