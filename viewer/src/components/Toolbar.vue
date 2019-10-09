<template>
  <div class="toolbar">
    <button class="scene-json" title="View scene JSON representation" />
    <button class="save-screenshot" title="Save current render" v-on:click="onSaveScreenshot()" />
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  @Prop() private onSaveScreenshot!: () => void;
}
</script>

<style scoped>
.toolbar {
  position: absolute;
  top: 0;
  right: 0;

  height: 32px;

  background-color: black;
  opacity: 0.8;
  display: flex;

  user-select: none;
}

.toolbar button {
  width: 32px;

  background-color: white;
  border: 1px solid black;
  background-size: cover;

  flex-grow: 0;
  flex-shrink: 0;
}

.toolbar button::-moz-focus-inner {
  border: 0;
}

.toolbar button:focus {
  border: 1px solid black;
  outline: none;
}

.toolbar button:active {
  box-shadow: inset 0px 0px 10px #c1c1c1;
}

.scene-json {
  background-image: url("../assets/scene-json.png");
}

.save-screenshot {
  background-image: url("../assets/save-screenshot.png");
}
</style>
