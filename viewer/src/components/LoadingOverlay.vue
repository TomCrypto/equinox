<template>
  <div v-show="isLoading" class="loading-overlay">
    <font-awesome-icon class="loading-icon" icon="circle-notch" spin size="lg" />
    <p>{{ loadingText }}</p>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  @Prop() private assetsInFlight!: number;

  get isLoading(): boolean {
    return true; // this.assetsInFlight != 0
  }

  get loadingText(): string {
    if (this.assetsInFlight > 1) {
      return `Downloading ${this.assetsInFlight} assets`;
    } else {
      return "Downloading 1 asset";
    }
  }
}
</script>

<style scoped>
.loading-overlay {
  position: absolute;
  bottom: 140px;
  left: 50%;

  transform: translateX(-50%);
  padding: 6px 6px;
  border-radius: 8px;
  height: 24px;

  background-color: black;
  opacity: 0.6667;
  display: flex;

  user-select: none;
  pointer-events: none;
  align-items: center;
}

.loading-icon {
  width: 24px;
  color: white;

  flex-grow: 0;
  flex-shrink: 0;
}

.loading-overlay p {
  color: #ffffff;
  padding: auto;
  font-size: 0.8em;
  line-height: 24px;
  font-family: monospace;
  font-weight: bold;

  flex-grow: 1;
  flex-shrink: 1;
  margin-left: 6px;
  margin-right: 2px;
}
</style>
