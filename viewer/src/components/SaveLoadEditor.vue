<template>
  <div class="scene-list">
    <div class="scene-card">
      <button class="button" v-on:click="saveScene()" title="Save the current scene">
        <div class="save-icon">
          <font-awesome-icon icon="hdd" size="2x" />
        </div>
      </button>
    </div>

    <div class="scene-card" v-for="scene in displayScenes" :key="scene.name">
      <button
        :style="`background-image: url(${scene.thumbnail})`"
        class="button scene-name-container"
        v-on:click="loadScene(scene.name)"
      >
        <div class="scene-name">{{ scene.name }}</div>
      </button>
      <div class="delete-scene" v-on:click="deleteScene(scene.name)">
        <font-awesome-icon class="delete-icon" icon="trash" size="lg" />
      </div>
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
    this.$root.$emit("save-scene-request");
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

.scene-card {
  display: flex;
  flex: 1;
  min-width: 320px;
  min-height: 180px;
  margin: 3px;
  position: relative;
}

.button {
  flex: 1;
  width: 100%;
  text-decoration: none;
  border-radius: 6px;
  background-position: center;
  background-repeat: no-repeat;
  background-size: contain;
  background-color: black;
  box-shadow: inset 0px 0px 5px 0px #555555;
  border: 1px solid #555555;
  cursor: pointer;
  padding-top: 4px;
  outline: 0;
}

.button::-moz-focus-inner {
  border: 0;
}

.button:active {
  position: relative;
  top: 2px;
}

.button:focus {
  box-shadow: 0 0 1pt 1pt #7193d9;
}

.scene-name-container {
  align-items: stretch;
  display: flex;
  flex-direction: column-reverse;
  position: relative;
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

.save-icon {
  color: #ffffff;
  font-size: 2em;
  flex: 0;
}

.delete-scene {
  cursor: pointer;
  display: inline-block;
  position: absolute;
  margin: 8px;
  top: 0;
  right: 0;
  width: 20px;
  height: 22px;
  background-color: transparent;
  color: #ffffff;
  border: 0;
  text-align: center;
  vertical-align: middle;
  z-index: 1;
}

.delete-icon:hover {
  color: #aaaaff;
  margin-top: 1px;
}

.delete-icon:active {
  color: #aaaaff;
  margin-top: 2px;
}
</style>
