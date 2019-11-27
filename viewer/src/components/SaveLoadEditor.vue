<template>
  <div class="scene-list">
    <button class="button" v-on:click="saveScene()">
      <div class="save-text">SAVE</div>
    </button>

    <button
      class="button load-scene"
      v-for="scene in displayScenes"
      :key="scene.name"
      :style="`background-image: url(${scene.thumbnail})`"
      v-on:click="loadScene(scene.name)"
    >
      <div class="scene-name">{{ scene.name }}</div>
    </button>
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
    name: "equinox-saved-scenes-v1"
  });

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

  private async saveScene() {
    this.$root.$emit("save-scene-request", "Unnamed scene");
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
.scene-list {
  flex: 1;
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;

  user-select: none;
}

.button {
  flex: 1;
  min-width: 320px;
  min-height: 180px;
  text-decoration: none;
  margin: 3px;
  background-color: black;
  border-radius: 6px;
  box-shadow: inset 0px 0px 5px 0px #555555;
  border: 1px solid #555555;
  cursor: pointer;
  padding-top: 4px;
  display: flex;
  flex-direction: column-reverse;
  align-items: stretch;
}

.load-scene {
  background-position: center;
  background-repeat: no-repeat;
  background-size: contain;
}

.button:active {
  position: relative;
  top: 2px;
}

.button:focus {
  box-shadow: 0 0 1pt 1pt #7193d9;
  outline: 0;
}

.scene-name {
  text-shadow: 0.08em 0 black, 0 0.08em black, -0.08em 0 black, 0 -0.08em black,
    -0.08em -0.08em black, -0.08em 0.08em black, 0.08em -0.08em black,
    0.08em 0.08em black;
  color: #ffffff;
  font-size: 1em;

  flex: 0;
  margin-bottom: 8px;
  margin-left: 4px;
  margin-right: 4px;
}

.save-text {
  color: #ffffff;
  font-size: 2em;
}
</style>
