<template>
  <div class="toolbar">
    <div
      class="toolbar-item"
      title="Toggle fullscreen mode"
      v-on:click="onToggleFullscreen()"
    >
      <font-awesome-icon class="toolbar-icon" icon="expand" size="2x" />
    </div>

    <div
      class="toolbar-item"
      title="Save the current render"
      v-on:click="onSaveRender()"
    >
      <font-awesome-icon
        v-if="!isSavingRender"
        class="toolbar-icon"
        icon="download"
        size="2x"
      />
      <font-awesome-icon
        v-if="isSavingRender"
        class="toolbar-icon"
        icon="cog"
        spin
        size="2x"
      />
    </div>

    <div
      v-if="!isRenderPaused"
      class="toolbar-item"
      title="Pause the render"
      v-on:click="toggleRenderPause()"
    >
      <font-awesome-icon class="toolbar-icon" icon="pause" size="2x" />
    </div>

    <div
      v-if="isRenderPaused"
      class="toolbar-item"
      title="Resume the render"
      v-on:click="toggleRenderPause()"
    >
      <font-awesome-icon class="toolbar-icon" icon="play" size="2x" />
    </div>

    <div
      v-if="!isCameraLocked"
      class="toolbar-item"
      title="Lock the camera"
      v-on:click="toggleCameraLock()"
    >
      <font-awesome-icon class="toolbar-icon" icon="unlock" size="2x" />
    </div>

    <div
      v-if="isCameraLocked"
      class="toolbar-item"
      title="Unlock the camera"
      v-on:click="toggleCameraLock()"
    >
      <font-awesome-icon class="toolbar-icon" icon="lock" size="2x" />
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  @Prop() private onToggleFullscreen!: () => void;
  @Prop() private onSaveRender!: () => void;
  @Prop() private isSavingRender!: boolean;
  @Prop() private isCameraLocked!: boolean;
  @Prop() private isRenderPaused!: boolean;

  private toggleCameraLock() {
    this.$emit("camera-lock");
  }

  private toggleRenderPause() {
    this.$emit("render-pause");
  }
}
</script>

<style scoped>
.toolbar {
  position: absolute;
  bottom: 56px;
  left: 50%;

  padding: 12px;

  transform: translateX(-50%);

  background-color: transparent;
  opacity: 0.1;
  filter: blur(1px);
  display: flex;

  transition: opacity 0.2s ease-out, filter 0.2s ease-out;
}

.toolbar:hover {
  filter: none;
  opacity: 0.8;
}

.toolbar-item:first-child {
  border-radius: 16px 0 0 16px;
  border-left-width: 4px;
}

.toolbar-item:last-child {
  border-radius: 0 16px 16px 0;
  border-right-width: 4px;
}

.toolbar-item {
  border-style: solid;
  border-color: #dddddd;
  border-top-width: 4px;
  border-bottom-width: 4px;
  border-left-width: 1px;
  border-right-width: 1px;

  width: 48px;
  height: 48px;
  color: #dddddd;

  background-color: #1a1a1a;

  transition: color 0.2s ease-out;

  cursor: pointer;

  flex-grow: 0;
  flex-shrink: 0;

  position: relative;
  display: flex;
  align-items: center;
}

.toolbar-item:hover {
  color: #438edf;
}

.toolbar-item:active {
  box-shadow: inset 0px 0px 5px #ffffff;
  color: #244d79;
}

.toolbar-icon {
  flex: 1;
}
</style>
