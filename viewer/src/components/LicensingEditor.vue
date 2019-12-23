<template>
  <div class="container">
    <div class="attribution">
      <p>
        All environment maps from
        <a href="https://hdrihaven.com/">HDRI Haven</a> and most textures from
        <a href="https://texturehaven.com/">Texture Haven</a>.
      </p>

      <hr />
    </div>

    <div class="licensing">
      <pre>{{ licensing }}</pre>
    </div>
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
  overflow: hidden;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.licensing {
  overflow: scroll;
  flex: 1;
  position: relative;
}

.licensing pre {
  height: 100%;
}

.attribution {
  flex: 0;
  text-align: center;
}

.attribution a {
  color: #dd5522;
}

.attribution a:hover {
  color: orange;
  text-decoration: underline;
}
</style>
