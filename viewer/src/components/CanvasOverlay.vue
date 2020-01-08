<template>
  <div>
    <div class="overlay" v-if="overlayType === 'error'">
      <font-awesome-icon class="icon error" icon="exclamation-triangle" size="lg" />
      <p class="error">{{ errorMessage }}</p>
    </div>

    <div class="overlay info" v-if="overlayType === 'download'">
      <font-awesome-icon class="icon" icon="circle-notch" spin size="lg" />
      <p class="info">{{ downloadText }}</p>
    </div>

    <div class="overlay info" v-if="overlayType === 'update'">
      <font-awesome-icon class="icon" icon="exclamation-circle" size="lg" />
      <p class="info">Updating scene...</p>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  @Prop() private assetsInFlight!: number;
  @Prop() private isExpensiveUpdate!: boolean;
  @Prop() private errorMessage!: string | null;

  public get overlayType(): "error" | "download" | "update" | null {
    if (this.errorMessage !== null) {
      return "error";
    }

    if (this.assetsInFlight !== 0) {
      return "download";
    }

    if (this.isExpensiveUpdate) {
      return "update";
    }

    return null;
  }

  public get downloadText(): string {
    if (this.assetsInFlight > 1) {
      return `Downloading ${this.assetsInFlight} assets`;
    } else {
      return "Downloading 1 asset";
    }
  }
}
</script>

<style scoped>
.overlay {
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

.icon {
  width: 24px;
  color: white;

  flex-grow: 0;
  flex-shrink: 0;
}

.overlay p {
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

.info {
  color: #ffffff;
}

.error {
  color: red;
}
</style>
