<template>
  <div>
    <div class="settings">
      <div class="settings-cell settings-label">Environment Type</div>
      <div class="settings-cell">
        <select
          :value="environmentType"
          @change="changeEnvironmentType($event.target.value)"
          selected
        >
          <option value="solid">Solid</option>
          <option value="map">Map</option>
        </select>
      </div>

      <div class="settings-cell settings-label">Environment Map</div>
      <div class="settings-cell">
        <select
          :value="environmentMap"
          :disabled="!isMapEnvironment"
          @change="changeEnvironmentMap($event.target.value)"
          selected
        >
          <option
            v-for="map in ENVIRONMENT_MAPS"
            :key="map.data"
            :value="map.data"
            >{{ map.name }}</option
          >
        </select>
      </div>

      <div class="settings-cell settings-label">Environment Tint (R)</div>
      <div class="settings-cell">
        <vue-slider
          :min="0"
          :max="1"
          tooltip="none"
          :interval="0.0001"
          :value="environmentTint[0]"
          contained="true"
          @change="changeEnvironmentTintR"
          @dragging="changeEnvironmentTintR"
        />
      </div>

      <div class="settings-cell settings-label">Environment Tint (G)</div>
      <div class="settings-cell">
        <vue-slider
          :min="0"
          :max="1"
          tooltip="none"
          :interval="0.0001"
          :value="environmentTint[1]"
          contained="true"
          @change="changeEnvironmentTintG"
          @dragging="changeEnvironmentTintG"
        />
      </div>

      <div class="settings-cell settings-label">Environment Tint (B)</div>
      <div class="settings-cell">
        <vue-slider
          :min="0"
          :max="1"
          tooltip="none"
          :interval="0.0001"
          :value="environmentTint[2]"
          contained="true"
          @change="changeEnvironmentTintB"
          @dragging="changeEnvironmentTintB"
        />
      </div>

      <div class="settings-cell settings-label">Environment Rotation</div>
      <div class="settings-cell">
        <vue-slider
          :min="0"
          :max="6.2832"
          :disabled="!isMapEnvironment"
          tooltip="none"
          :interval="0.0001"
          :value="environmentRotation"
          contained="true"
          @change="changeEnvironmentRotation"
          @dragging="changeEnvironmentRotation"
        />
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene } from "equinox";

type SceneEnvironment =
  | {
      type: "solid";
      tint: [number, number, number];
    }
  | {
      type: "map";
      tint: [number, number, number];
      rotation: number;
    };

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;

  public changeEnvironmentMap(value: string) {
    this.environmentMap = value;
    this.update();
  }

  private changeEnvironmentType(value: "solid" | "map") {
    this.environmentType = value;

    if (this.environmentType === "map" && this.environmentMap === null) {
      this.environmentMap = this.ENVIRONMENT_MAPS[0].data;
    }

    this.update();
  }

  public changeEnvironmentRotation(value: number) {
    this.environmentRotation = value;
    this.update();
  }

  public changeEnvironmentTintR(value: number) {
    this.environmentTint[0] = value;
    this.update();
  }

  public changeEnvironmentTintG(value: number) {
    this.environmentTint[1] = value;
    this.update();
  }

  public changeEnvironmentTintB(value: number) {
    this.environmentTint[2] = value;
    this.update();
  }

  public get isMapEnvironment() {
    return this.environmentType === "map";
  }

  ENVIRONMENT_MAPS = [
    {
      name: "Aft Lounge (1K)",
      data: "aft_lounge_1k.raw"
    },
    {
      name: "Aft Lounge (2K)",
      data: "aft_lounge_2k.raw"
    },
    {
      name: "Aft Lounge (4K)",
      data: "aft_lounge_4k.raw"
    },
    {
      name: "Aft Lounge (8K)",
      data: "aft_lounge_8k.raw"
    },
    {
      name: "Bethnal Green Entrance (4K)",
      data: "bethnal_green_entrance_4k.raw"
    },
    {
      name: "Between Bridges (1K)",
      data: "between_bridges_1k.raw"
    },
    {
      name: "Between Bridges (2K)",
      data: "between_bridges_2k.raw"
    },
    {
      name: "Between Bridges (4K)",
      data: "between_bridges_4k.raw"
    },
    {
      name: "Between Bridges (8K)",
      data: "between_bridges_8k.raw"
    },
    {
      name: "Blue Grotto (4K)",
      data: "blue_grotto_4k.raw"
    },
    {
      name: "Blue Lagoon Night (4K)",
      data: "blue_lagoon_night_4k.raw"
    },
    {
      name: "Blue Lagoon Night (8K)",
      data: "blue_lagoon_night_8k.raw"
    },
    {
      name: "Carpentry Shop 02 (4K)",
      data: "carpentry_shop_02_4k.raw"
    },
    {
      name: "Cayley Interior (8K)",
      data: "cayley_interior_8k.raw"
    },
    {
      name: "Cinema Lobby (4K)",
      data: "cinema_lobby_4k.raw"
    },
    {
      name: "Cinema Lobby (8K)",
      data: "cinema_lobby_8k.raw"
    },
    {
      name: "Green Point Park (8K)",
      data: "green_point_park_8k.raw"
    },
    {
      name: "Lenong 2 (1K)",
      data: "lenong_2_1k.raw"
    },
    {
      name: "Lenong 2 (2K)",
      data: "lenong_2_2k.raw"
    },
    {
      name: "Lenong 2 (4K)",
      data: "lenong_2_4k.raw"
    },
    {
      name: "Moonless Golf (4K)",
      data: "moonless_golf_4k.raw"
    },
    {
      name: "Moonless Golf (8K)",
      data: "moonless_golf_8k.raw"
    },
    {
      name: "Noon Grass (2K)",
      data: "noon_grass_2k.raw"
    },
    {
      name: "Noon Grass (4K)",
      data: "noon_grass_4k.raw"
    },
    {
      name: "Noon Grass (8K)",
      data: "noon_grass_8k.raw"
    },
    {
      name: "Old Outdoor Theater (4K)",
      data: "old_outdoor_theater_4k.raw"
    },
    {
      name: "Paul Lobe Haus (4K)",
      data: "paul_lobe_haus_4k.raw"
    },
    {
      name: "Sunny Vondelpark (2K)",
      data: "sunny_vondelpark_2k.raw"
    },
    {
      name: "Sunny Vondelpark (4K)",
      data: "sunny_vondelpark_4k.raw"
    },
    {
      name: "Sunny Vondelpark (8K)",
      data: "sunny_vondelpark_8k.raw"
    }
  ];

  // This editor's internal view of the scene.

  environmentType: "solid" | "map" = "solid";
  environmentMap: string | null = null;
  environmentTint: [number, number, number] = [1, 1, 1];
  environmentRotation: number = 0;

  created() {
    const [json, environment] = this.getSceneData();

    this.environmentType = environment.type;
    this.environmentTint = environment.tint;

    if (environment.type === "map") {
      this.environmentRotation = environment.rotation;
    }

    this.environmentMap = json.environment_map;
  }

  private update() {
    const [json, environment] = this.getSceneData();

    environment.type = this.environmentType;
    environment.tint = this.environmentTint;

    if (environment.type === "map") {
      environment.rotation = this.environmentRotation;
    }

    json.environment_map = this.environmentMap;

    this.scene.set_json(json);
  }

  private getSceneData(): [any, SceneEnvironment] {
    const json = this.scene.json();

    return [json, json.environment];
  }
}
</script>

<style scoped>
.settings {
  display: grid;
  grid-template-columns: min-content auto;
  white-space: nowrap;
  border-top: 2px solid #333333;
  border-left: 2px solid #333333;
}

.settings-cell {
  padding: 5px;
  border-bottom: 2px solid #333333;
  border-right: 2px solid #333333;
}

.settings-label {
  padding: 5px 8px;
  text-align: right;
}

.settings-label-disabled {
  color: #888888;
}

select {
  width: 100%;
}

input {
  width: 100%;
}
</style>
