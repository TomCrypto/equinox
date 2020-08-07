<template>
  <div>
    <div class="settings">
      <div class="settings-cell settings-label">Field of View</div>
      <div class="settings-cell">
        <vue-slider
          :min="0.01"
          :max="1.0"
          tooltip="none"
          :interval="0.001"
          :value="fieldOfView"
          contained="true"
          @change="changeFieldOfView"
          @dragging="changeFieldOfView"
        />
      </div>
      <div class="settings-cell settings-label">Focal Curvature</div>
      <div class="settings-cell">
        <vue-slider
          :min="0"
          :max="1"
          tooltip="none"
          :interval="0.001"
          :value="focalCurvature"
          contained="true"
          @change="changeFocalCurvature"
          @dragging="changeFocalCurvature"
        />
      </div>
      <div class="settings-cell settings-label">Aperture Type</div>
      <div class="settings-cell">
        <select
          :value="apertureType"
          @change="changeApertureType($event.target.value)"
          selected
        >
          <option value="point">Point</option>
          <option value="circle">Circle</option>
          <option value="ngon">Polygon</option>
        </select>
      </div>
      <div
        class="settings-cell settings-label"
        v-bind:class="{ 'settings-label-disabled': !hasApertureRadius }"
      >
        Aperture Radius
      </div>
      <div class="settings-cell">
        <vue-slider
          :min="0"
          :max="1"
          :disabled="!hasApertureRadius"
          tooltip="none"
          :interval="0.001"
          :value="apertureRadius"
          contained="true"
          @change="changeApertureRadius"
          @dragging="changeApertureRadius"
        />
      </div>
      <div
        class="settings-cell settings-label"
        v-bind:class="{ 'settings-label-disabled': !hasApertureRadius }"
      >
        Focal Distance
      </div>
      <div class="settings-cell">
        <vue-slider
          :min="0.01"
          :max="100"
          :disabled="!hasApertureRadius"
          tooltip="none"
          :interval="0.001"
          :value="focalDistance"
          contained="true"
          @change="changeFocalDistance"
          @dragging="changeFocalDistance"
        />
      </div>
      <div
        class="settings-cell settings-label"
        v-bind:class="{ 'settings-label-disabled': !isPolygonAperture }"
      >
        Aperture Rotation
      </div>
      <div class="settings-cell">
        <vue-slider
          :min="0"
          :max="6.2832"
          :disabled="!isPolygonAperture"
          tooltip="none"
          :interval="0.0001"
          :value="apertureRotation"
          contained="true"
          @change="changeApertureRotation"
          @dragging="changeApertureRotation"
        />
      </div>
      <div
        class="settings-cell settings-label"
        v-bind:class="{ 'settings-label-disabled': !isPolygonAperture }"
      >
        Aperture Sides
      </div>
      <div class="settings-cell">
        <vue-slider
          :min="3"
          :max="12"
          :disabled="!isPolygonAperture"
          tooltip="none"
          :interval="1"
          :adsorb="true"
          :value="apertureSides"
          contained="true"
          @change="changeApertureSides"
          @dragging="changeApertureSides"
        />
      </div>
      <div class="settings-cell settings-label">Lens Flare</div>
      <div class="settings-cell">
        <select
          :value="lensFlareEnabled ? 'enabled' : 'disabled'"
          @change="changeLensFlareEnabled($event.target.value)"
          selected
        >
          <option value="disabled">Disabled</option>
          <option value="enabled">Enabled</option>
        </select>
      </div>
      <div class="settings-cell settings-label">Aperture Filter</div>
      <div class="settings-cell">
        <select
          :value="apertureFilter"
          :disabled="!lensFlareEnabled"
          @change="changeApertureFilter($event.target.value)"
          selected
        >
          <option
            v-for="map in APERTURE_FILTERS"
            :key="map.data"
            :value="map.data"
            >{{ map.name }}</option
          >
        </select>
      </div>
      <div class="settings-cell settings-label">Aperture Filter Tile Size</div>
      <div class="settings-cell">
        <select
          :value="apertureFilterTileSize"
          :disabled="!lensFlareEnabled"
          @change="changeApertureFilterTileSize($event.target.value)"
          selected
        >
          <option value="lowest">Lowest</option>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene } from "equinox";

interface SceneCamera {
  aperture:
    | {
        type: "point";
      }
    | {
        type: "circle";
        radius: number;
      }
    | {
        type: "ngon";
        radius: number;
        sides: number;
        rotation: number;
      };
  focal_distance: number;
  focal_curvature: number;
  field_of_view: number;
}

interface SceneDisplay {
  lens_flare_enabled: boolean;
}

interface SceneAperture {
  filter: string;
  tile_size: "lowest" | "low" | "medium" | "high";
}

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;

  private changeApertureType(value: "point" | "circle" | "ngon") {
    this.apertureType = value;
    this.update();
  }

  private changeLensFlareEnabled(value: "disabled" | "enabled") {
    this.lensFlareEnabled = value === "enabled";

    if (this.lensFlareEnabled && this.apertureFilter === null) {
      this.apertureFilter = this.APERTURE_FILTERS[0].data;
    }

    this.update();
  }

  private changeApertureFilter(value: string) {
    this.apertureFilter = value;
    this.update();
  }

  private changeApertureFilterTileSize(value: "lowest" | "low" | "medium" | "high") {
    this.apertureFilterTileSize = value;
    this.update();
  }

  public get hasApertureRadius() {
    return this.apertureType !== "point";
  }

  public get isPolygonAperture() {
    return this.apertureType === "ngon";
  }

  public changeApertureRadius(value: number) {
    this.apertureRadius = value;
    this.update();
  }

  public changeFocalDistance(value: number) {
    this.focalDistance = value;
    this.update();
  }

  public changeApertureRotation(value: number) {
    this.apertureRotation = value;
    this.update();
  }

  public changeApertureSides(value: number) {
    this.apertureSides = value;
    this.update();
  }

  public changeFocalCurvature(value: number) {
    this.focalCurvature = value;
    this.update();
  }

  public changeFieldOfView(value: number) {
    this.fieldOfView = value;
    this.update();
  }

  // This editor's internal view of the scene.

  apertureType: "point" | "circle" | "ngon" = "point";
  apertureRadius: number = 0;
  apertureSides: number = 5;
  apertureRotation: number = 0;
  focalDistance: number = 0;
  focalCurvature: number = 0;
  fieldOfView: number = 0;
  lensFlareEnabled: boolean = false;
  apertureFilter: string | null = null;
  apertureFilterTileSize: "lowest" | "low" | "medium" | "high" = "low";

  created() {
    const [_, camera, aperture, display] = this.getSceneData();

    this.apertureType = camera.aperture.type;

    switch (camera.aperture.type) {
      case "circle":
        this.apertureRadius = camera.aperture.radius;
        break;
      case "ngon":
        this.apertureRadius = camera.aperture.radius;
        this.apertureSides = camera.aperture.sides;
        this.apertureRotation = camera.aperture.rotation;
        break;
    }

    this.focalDistance = camera.focal_distance;
    this.focalCurvature = camera.focal_curvature;
    this.fieldOfView = camera.field_of_view;

    this.lensFlareEnabled = display.lens_flare_enabled;
    this.apertureFilter = aperture ? aperture.filter : null;
    this.apertureFilterTileSize = aperture ? aperture.tile_size : "low";
  }

  private update() {
    const [json, camera, aperture, display] = this.getSceneData();

    camera.aperture.type = this.apertureType;

    switch (camera.aperture.type) {
      case "circle":
        camera.aperture.radius = this.apertureRadius;
        break;
      case "ngon":
        camera.aperture.radius = this.apertureRadius;
        camera.aperture.sides = this.apertureSides;
        camera.aperture.rotation = this.apertureRotation;
        break;
    }

    camera.focal_distance = this.focalDistance;
    camera.focal_curvature = this.focalCurvature;
    camera.field_of_view = this.fieldOfView;

    display.lens_flare_enabled = this.lensFlareEnabled;

    if (this.apertureFilter === null) {
      json.aperture = null;
    } else {
      json.aperture = {
        filter: this.apertureFilter,
        tile_size: this.apertureFilterTileSize,
      }
    }

    this.scene.set_json(json);
  }

  private getSceneData(): [any, SceneCamera, SceneAperture | null, SceneDisplay] {
    const json = this.scene.json();

    return [json, json.camera, json.aperture, json.display];
  }

  APERTURE_FILTERS = [
    {
      name: "Aperture Filter 1",
      data: "aperture_filter_1.raw"
    },
    {
      name: "Aperture Filter 2",
      data: "aperture_filter_2.raw"
    },
    {
      name: "Aperture Filter 3",
      data: "aperture_filter_3.raw"
    },
  ]
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
</style>
