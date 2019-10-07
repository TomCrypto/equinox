<template>
  <div id="app">
    <Viewport :equinox="equinox" :scene="scene" />
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import Viewport from "./components/Viewport.vue";
import { WebScene } from "equinox";

async function fetch_bytes(url: string) {
  return (await fetch(new Request(url))).arrayBuffer();
}

@Component({
  components: {
    Viewport
  }
})
export default class App extends Vue {
  @Prop() private equinox!: typeof import("equinox");

  private scene!: WebScene;

  created() {
    this.scene = new this.equinox.WebScene();
    this.scene.setup_test_scene();

    console.log("Fetching envmap...");

    (async () => {
      let data = new Uint8Array(await fetch_bytes("assets/blue_grotto_4k.raw"));

      console.log("Fetched envmap data: " + data.length + " pixels");

      this.scene.insert_asset("envmap", data);
      this.scene.set_envmap("envmap", 4096, 2048);

      console.log("Inserted into scene!");
    })();
  }
}
</script>

<style>
body {
  margin: 0;
}
</style>
