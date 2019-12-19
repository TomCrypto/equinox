<template>
  <div class="container">
    <pre class="licensing">{{ licensing }}</pre>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  @Prop() private licensing!: string;
}
</script>

<style scoped>
.container {
  display: flex;
  flex-direction: column;
}

.licensing {
  flex: 1;
}
</style>
