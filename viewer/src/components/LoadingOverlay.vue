<template>
  <div v-show="isLoading" class="loading-overlay">
    <img src="@/assets/loading-assets.svg" />
    <p>{{ loadingText }}</p>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  // number of assets currently being loaded
  @Prop() private loadingCount!: number;
  // of those, number of assets currently downloading
  @Prop() private downloadingCount!: number;

  get isLoading(): boolean {
    return this.loadingCount != 0 || this.downloadingCount != 0;
  }

  get loadingText(): string {
    if (this.downloadingCount > 0) {
      return `Downloading ${this.pluralizedAsset(this.downloadingCount)}`;
    } else {
      return `Loading ${this.pluralizedAsset(this.loadingCount)}`;
    }
  }

  private pluralizedAsset(count: number): string {
    return count === 1 ? "1 asset" : `${count} assets`;
  }
}
</script>

<style scoped>
.loading-overlay {
  position: absolute;
  top: 92%;
  left: 50%;

  transform: translate(-50%, -50%);
  padding: 6px 6px;
  border-radius: 8px;
  height: 24px;

  background-color: black;
  opacity: 0.6667;
  display: flex;

  user-select: none;
  pointer-events: none;
}

.loading-overlay img {
  width: 24px;

  margin-right: 6px;

  flex-grow: 0;
  flex-shrink: 0;
}

.loading-overlay p {
  color: #ffffff;
  margin: 0;
  padding: auto;
  font-size: 0.8em;
  line-height: 24px;
  font-family: monospace;
  font-weight: bold;

  flex-grow: 1;
  flex-shrink: 1;
  margin-right: 6px;
}
</style>
