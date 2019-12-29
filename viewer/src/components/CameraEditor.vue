<template>
  <div>
    <p>Aperture</p>

    <div class="radio-group">
      <input
        type="radio"
        id="point"
        name="selector"
        @click="selectPointAperture()"
        :checked="apertureType == 'point'"
      />
      <label for="point">Point</label>
      <input
        type="radio"
        id="circle"
        name="selector"
        @click="selectCircleAperture()"
        :checked="apertureType == 'circle'"
      />
      <label for="circle">Circle</label>
      <input
        type="radio"
        id="polygon"
        name="selector"
        @click="selectPolygonAperture()"
        :checked="apertureType == 'ngon'"
      />
      <label for="polygon">Polygon</label>
    </div>

    <p>Aperture Radius</p>

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

    <p>Focal distance</p>

    <vue-slider
      :min="0.001"
      :max="100"
      tooltip="none"
      :interval="0.001"
      :value="focalDistance"
      contained="true"
      @change="changeFocalDistance"
      @dragging="changeFocalDistance"
    />
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
}

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;

  public selectPointAperture() {
    this.apertureType = "point";
    this.update();
  }

  public selectCircleAperture() {
    this.apertureType = "circle";
    this.update();
  }

  public selectPolygonAperture() {
    this.apertureType = "ngon";
    this.update();
  }

  public get hasApertureRadius() {
    return this.apertureType != "point";
  }

  public changeApertureRadius(value: number) {
    this.apertureRadius = value;
    this.update();
  }

  public changeFocalDistance(value: number) {
    this.focalDistance = value;
    this.update();
  }

  // This editor's internal view of the scene.

  apertureType: "point" | "circle" | "ngon" = "point";
  apertureRadius: number = 0;
  apertureSides: number = 5;
  apertureRotation: number = 0;
  focalDistance: number = 0;

  created() {
    const [_, camera] = this.getSceneData();

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
  }

  private update() {
    const [json, camera] = this.getSceneData();

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

    this.scene.set_json(json);
  }

  private getSceneData(): [any, SceneCamera] {
    const json = this.scene.json();

    return [json, json.camera as SceneCamera];
  }
}
</script>

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
