<template>
  <div class="dark-overlay" ref="overlay" tabindex="0" v-on:keydown="onKeyDown($event)">
    <div class="download-overlay">
      <a :href="downloadUrl" download="render.zip" v-on:click="onClose()">
        <p>Click here to download your render ({{ downloadSize }})</p>
      </a>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  @Prop() private render!: Blob | null;
  @Prop() private onClose!: () => void;

  get downloadUrl(): string {
    if (this.render === null) {
      return ""; // no render
    }

    return URL.createObjectURL(this.render);
  }

  get downloadSize(): string {
    if (this.render === null) {
      return ""; // no render
    }

    return `${(this.render.size / (1024 * 1024)).toFixed(1)} MiB`;
  }

  mounted() {
    (this.$refs.overlay as HTMLDivElement).focus();
  }

  private onClick(event: Event) {
    event.stopPropagation();
    this.closeOverlay();
  }

  private onKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      this.closeOverlay();
    }
  }

  private closeOverlay() {
    URL.revokeObjectURL(this.downloadUrl);
    this.onClose(); // let owner close us
  }
}
</script>

<style scoped>
.dark-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  opacity: 0.95;
  background-color: black;

  outline: none;
}

.download-overlay {
  position: absolute;
  top: 50%;
  left: 50%;

  transform: translate(-50%, -50%);
  padding: 6px 6px;
  border-radius: 8px;
  height: 24px;

  background-color: black;
  opacity: 0.6667;
}

.download-overlay a {
  text-decoration: none;
}

.download-overlay p {
  color: #ffffff;
  padding: auto;
  font-size: 1.2em;
  font-family: monospace;
  font-weight: bold;

  user-select: none;

  padding: 5px;

  border: 2px solid white;
  text-align: center;
}
</style>
