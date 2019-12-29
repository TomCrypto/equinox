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
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene } from "equinox";

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

  // This editor's internal view of the scene. The default values below
  // are only specified to allow Vue to see the properties as reactive.

  apertureType: "point" | "circle" | "ngon" = "point";
  apertureRadius: number = 0;
  apertureSides: number = 0;
  apertureRotation: number = 0;

  created() {
    const json = this.scene.json();

    this.apertureType = json.camera.aperture.type;

    if (["circle", "ngon"].includes(this.apertureType)) {
      this.apertureRadius = json.camera.aperture.radius;
    } else {
      this.apertureRadius = 0;
    }

    if (["ngon"].includes(this.apertureType)) {
      this.apertureSides = json.camera.aperture.sides;
      this.apertureRotation = json.camera.aperture.rotation;
    } else {
      this.apertureSides = 5;
      this.apertureRotation = 0;
    }
  }

  private update() {
    const json = this.scene.json();

    json.camera.aperture.type = this.apertureType;

    if (["circle", "ngon"].includes(this.apertureType)) {
      json.camera.aperture.radius = this.apertureRadius;
    }

    if (["ngon"].includes(this.apertureType)) {
      json.camera.aperture.sides = this.apertureSides;
      json.camera.aperture.rotation = this.apertureRotation;
    }

    this.scene.set_json(json);
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
