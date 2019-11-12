<template>
  <div class="editor">
    <multiselect
      :value="environmentMap"
      :options="environmentMaps"
      :show-labels="false"
      @input="onSelectEnvironmentMap"
    ></multiselect>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene } from "equinox";

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;
  @Prop() private loadAsset!: (url: string) => Promise<void>;

  private sceneJson: any = null;

  private get environmentMap(): string | null {
    const environment: string | null = this.sceneJson.environment_map;

    if (environment === null) {
      return null;
    }

    return this.getMapForUrl(environment);
  }

  private get environmentMaps(): string[] {
    return this.MAPS.map(map => map.name);
  }

  private onSelectEnvironmentMap(map: string) {
    this.updateEnvironmentMap(this.getUrlForMap(map));
  }

  private async updateEnvironmentMap(url: string) {
    await this.loadAsset(url);

    this.scene.set_envmap(url);
    this.sceneJson = this.scene.json();
  }

  private getUrlForMap(name: string): string {
    for (const map of this.MAPS) {
      if (map.name === name) {
        return map.url;
      }
    }

    throw new Error("bad environment map");
  }

  private getMapForUrl(url: string): string {
    for (const map of this.MAPS) {
      if (map.url === url) {
        return map.name;
      }
    }

    throw new Error("bad environment map");
  }

  created() {
    this.sceneJson = this.scene.json();
  }

  MAPS = [
    {
      name: "Blue Grotto (4K)",
      url: "assets/blue_grotto_4k.raw"
    },
    {
      name: "Paul Lobe Haus (4K)",
      url: "assets/paul_lobe_haus_4k.raw"
    },
    {
      name: "Bethnal Green Entrance (4K)",
      url: "assets/bethnal_green_entrance_4k.raw"
    },
    {
      name: "Cayley Interior (8K)",
      url: "assets/cayley_interior_8k.raw"
    },
    {
      name: "Green Point Park (8K)",
      url: "assets/green_point_park_8k.raw"
    },
    {
      name: "Carpentry Shop 02 (4K)",
      url: "assets/carpentry_shop_02_4k.raw"
    },
    {
      name: "Old Outdoor Theater (4K)",
      url: "assets/old_outdoor_theater_4k.raw"
    }
  ];
}
</script>

<style src="vue-multiselect/dist/vue-multiselect.min.css"></style>

<style scoped>
.editor {
  position: absolute;

  top: 10%;
  left: 75%;

  width: 20%;
  height: 80%;

  border: 2px solid black;
  border-radius: 5px;
}
</style>
