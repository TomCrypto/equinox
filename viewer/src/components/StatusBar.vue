<template>
  <div class="status">
    <div class="frame-rate">
      <pre><p>{{ frameRate }}</p></pre>
    </div>
    <div class="resolution">
      <pre><p>{{ width }}×{{ height }}</p></pre>
    </div>
    <div class="pass-info">
      <pre><p>{{ passInfo }}</p></pre>
    </div>
    <div class="frame-cpu-time">
      <pre><p>{{ cpuFrameInfo }}</p></pre>
    </div>
    <div class="frame-gpu-time">
      <pre><p>{{ gpuFrameInfo }}</p></pre>
    </div>
    <div class="device-info">
      <pre><p>{{ renderer }} ({{ vendor }})</p></pre>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

function displayTime(milliseconds: number): string {
  if (milliseconds <= 0.099) {
    return `${(milliseconds * 1000.0).toFixed(0).padStart(4, " ")} μs`;
  }

  if (milliseconds <= 99) {
    return `${milliseconds.toFixed(1).padStart(4, " ")} ms`;
  }

  return `${milliseconds.toFixed(0).padStart(4, " ")} ms`;
}

function displayPhotons(amount: number): string {
  if (amount === 1) {
    return "1 photon";
  }

  if (amount < 1000) {
    return `${amount} photons`;
  }

  if (amount < 1e6) {
    return `${(amount / 1e3).toFixed(1)}K photons`;
  }

  if (amount < 1e9) {
    return `${(amount / 1e6).toFixed(1)}M photons`;
  }

  if (amount < 1e12) {
    return `${(amount / 1e9).toFixed(1)}B photons`;
  }

  return `${(amount / 1e12).toFixed(1)}T photons`;
}

@Component
export default class extends Vue {
  @Prop() private width!: number;
  @Prop() private height!: number;
  @Prop() private sppmPasses!: number;
  @Prop() private sppmPhotons!: number;
  @Prop() private vendor!: string;
  @Prop() private renderer!: string;
  @Prop() private isContextLost!: boolean;

  @Prop() private cpuFrameTime!: number | null;
  @Prop() private gpuFrameTime!: number | null;
  @Prop() private syncInterval!: number | null;

  get passInfo(): string {
    if (this.isContextLost) {
      return "CONTEXT LOST!";
    } else if (this.sppmPasses === 1) {
      return `1 pass, ${displayPhotons(this.sppmPhotons)}`;
    } else {
      return `${this.sppmPasses} passes, ${displayPhotons(this.sppmPhotons)}`;
    }
  }

  get frameRate(): string {
    if (this.syncInterval === null) {
      return "-- FPS";
    }

    return `${(1000 / this.syncInterval).toFixed(0)} FPS`;
  }

  get cpuFrameInfo(): string {
    if (this.cpuFrameTime === null) {
      return "CPU time:   N/A  ";
    }

    return `CPU time: ${displayTime(this.cpuFrameTime * 1000)}`;
  }

  get gpuFrameInfo(): string {
    if (this.gpuFrameTime === null) {
      return "GPU time:   N/A  ";
    }

    return `GPU time: ${displayTime(this.gpuFrameTime * 1000)}`;
  }
}
</script>

<style scoped>
.status {
  background-color: black;
  opacity: 0.6667;
  display: flex;

  flex-wrap: wrap;
  overflow: hidden;
  flex-grow: 0;
  flex-shrink: 0;
  user-select: none;
}

.status > :not(:last-child) {
  border-right: 1px solid #777777;
  text-align: center;
}

.status > :last-child {
  text-align: left;
  margin-left: 6px;
  flex-grow: 1;
  flex-shrink: 1;
}

.frame-rate {
  width: 70px;
}

.pass-info {
  width: 230px;
}

.resolution {
  width: 80px;
}

.frame-cpu-time {
  width: 120px;
}

.frame-gpu-time {
  width: 120px;
}

p {
  color: #ffffff;
  margin: 0;
  padding: auto;
  font-size: 0.8em;
  line-height: 18px;
  font-family: monospace;
  font-weight: bold;
}

pre {
  margin: 0;
}
</style>
