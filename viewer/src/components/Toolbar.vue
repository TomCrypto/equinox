<template>
  <div class="toolbar">
    <button
      class="toggle-fullscreen"
      title="Toggle fullscreen mode"
      v-on:click="onToggleFullscreen()"
    />

    <button class="save-render" title="Save the current render" v-on:click="onSaveRender()" />

    <button
      v-if="!isCameraLocked"
      class="lock-camera"
      title="Lock camera"
      v-on:click="toggleCameraLock()"
    />
    <button
      v-if="isCameraLocked"
      class="unlock-camera"
      title="Unlock camera"
      v-on:click="toggleCameraLock()"
    />
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  @Prop() private onToggleCameraLock!: (locked: boolean) => void;
  @Prop() private onSaveRender!: () => void;
  @Prop() private onToggleFullscreen!: () => void;

  private isCameraLocked: boolean = false;

  private toggleCameraLock() {
    this.isCameraLocked = !this.isCameraLocked;
    this.onToggleCameraLock(this.isCameraLocked);
  }
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

.toggle-fullscreen {
  background-image: url("../assets/toggle-fullscreen.png");
}

.lock-camera {
  background-image: url("../assets/lock-camera.png");
}

.unlock-camera {
  background-image: url("../assets/unlock-camera.png");
}

.save-render {
  background-image: url("../assets/save-render.png");
}
</style>
