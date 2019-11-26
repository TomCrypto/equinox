<template>
  <div>
    <div class="radio-group">
      <input type="radio" id="solid" name="selector" />
      <label for="solid">Solid Background</label>
      <input type="radio" id="map" name="selector" />
      <label for="map">Environment Map</label>
    </div>

    <multiselect
      :value="environmentMap"
      :options="environmentMaps"
      :show-labels="false"
      @input="onSelectEnvironmentMap"
    />
    <vue-slider
      :min="0"
      :max="1"
      tooltip="none"
      :interval="0.001"
      @change="changeRotation"
      @dragging="changeRotation"
    />
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene } from "equinox";

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;
  @Prop() private loadAssets!: (assets: string[]) => Promise<void>;

  private sceneJson: any = null;

  private changeRotation(value: number) {
    this.scene.set_environment_rotation(value * 2.0 * Math.PI);
  }

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
    const oldEnvmap = this.sceneJson.environment_map as string | null;

    await this.loadAssets([url]);

    this.scene.set_envmap(url);
    this.sceneJson = this.scene.json();

    if (oldEnvmap !== null) {
      this.scene.remove_asset(oldEnvmap);
    }
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
    },
    {
      name: "Blue Lagoon Night (4K)",
      url: "assets/blue_lagoon_night_4k.raw"
    },
    {
      name: "Blue Lagoon Night (8K)",
      url: "assets/blue_lagoon_night_8k.raw"
    },
    {
      name: "Cinema Lobby (4K)",
      url: "assets/cinema_lobby_4k.raw"
    },
    {
      name: "Cinema Lobby (8K)",
      url: "assets/cinema_lobby_8k.raw"
    },
    {
      name: "Moonless Golf (4K)",
      url: "assets/moonless_golf_4k.raw"
    },
    {
      name: "Moonless Golf (8K)",
      url: "assets/moonless_golf_8k.raw"
    }
  ];
}
</script>

<style src="vue-multiselect/dist/vue-multiselect.min.css"></style>

<style scoped>
input[type="radio"] {
  position: absolute;
  visibility: hidden;
  display: none;
}

label {
  color: #332f35;
  display: inline-block;
  cursor: pointer;
  font-weight: bold;
  padding: 5px 20px;
}

input[type="radio"]:checked + label {
  color: #132f35;
  background: #332f35;
}

label + input[type="radio"] + label {
  border-left: solid 3px #332f35;
}
.radio-group {
  border: solid 3px #332f35;
  display: inline-block;
  margin: 20px;
  border-radius: 10px;
  overflow: hidden;
}
</style>
