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
        v-on:click="loadScene(scene.name)"
      >{{ scene.name }}</button>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import localforage from "localforage";
import { WebScene } from "equinox";

interface Metadata {
  thumbnail: string;
  json: object;
  assets: string[];
}

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;

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
      scenes.push({ name, scene });
    }

    return scenes;
  }

  mounted() {
    this.updateFromStore();
  }

  // how to get screenshot from current scene here??
  private async saveScene(name: string) {
    // TODO: fetch the current scene JSON and create a thumbnail of the render
    // this needs to talk to the canvas container, so we'll bubble up and then
    // back down

    console.log("Saving " + name);

    const scene = {
      thumbnail: "",
      json: this.scene.json(),
      assets: this.scene.assets()
    };

    await this.store.setItem(name, scene);
    this.updateFromStore();
  }

  private async deleteScene(name: string) {
    await this.store.removeItem(name);
    this.updateFromStore();
  }

  private async loadScene(name: string) {
    console.log("Loading scene " + name);

    const scene = this.scenes.get(name);

    if (scene === undefined) {
      throw new Error("scene did not exist");
    }

    // TODO: load assets into the scene, then remove all old unneeded assets
    // (this will need a "load_assets" callback of some kind)

    this.scene.set_json(scene.json);
  }

  private async updateFromStore() {
    const scenes = new Map();

    for (const name of await this.store.keys()) {
      scenes.set(name, await this.store.getItem(name));
    }

    this.scenes = scenes;
    console.log(this.scenes);

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
  flex-direction: column;
}

.load-scene {
  flex: 1;
}
</style>
