<template>
  <div class="status">
    <div class="frame-rate">
      <pre><p>{{ frameRate }}</p></pre>
    </div>
    <div class="resolution">
      <pre><p>{{ width }}×{{ height }}</p></pre>
    </div>
    <div class="sample-count">
      <pre><p>{{ sampleInfo }}</p></pre>
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

@Component
export default class extends Vue {
  @Prop() private width!: number;
  @Prop() private height!: number;
  @Prop() private sampleCount!: number;
  @Prop() private vendor!: string;
  @Prop() private renderer!: string;
  @Prop() private isContextLost!: boolean;

  @Prop() private cpuFrameTime!: number | null;
  @Prop() private gpuFrameTime!: number | null;
  @Prop() private syncInterval!: number | null;

  get sampleInfo(): string {
    if (this.isContextLost) {
      return "CONTEXT LOST!";
    } else if (this.sampleCount == 1) {
      return `1 sample`;
    } else {
      return `${this.sampleCount} samples`;
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

.sample-count {
  width: 130px;
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
