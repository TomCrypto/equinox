<template>
  <div class="container">
    <header class="container-header header-above">
      <ul class="tab-heads">
        <li
          class="tab-head above"
          v-for="tab in tabsAbove"
          :key="tab"
          v-bind:class="{
            'tab-head--active': activeTab === tab
          }"
          v-on:click="switchTab(tab)"
        >
          <slot :name="`tab-head-${tab}`">{{ tab }}</slot>
        </li>
      </ul>
    </header>
    <main class="container-body">
      <div class="tab-panel">
        <slot :name="`tab-panel-${this.activeTab}`"></slot>
      </div>
    </main>
    <header class="container-header header-below">
      <ul class="tab-heads">
        <li
          class="tab-head below"
          v-for="tab in tabsBelow"
          :key="tab"
          v-bind:class="{
            'tab-head--active': activeTab === tab
          }"
          v-on:click="switchTab(tab)"
        >
          <slot :name="`tab-head-${tab}`">{{ tab }}</slot>
        </li>
      </ul>
    </header>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";

@Component
export default class extends Vue {
  @Prop() private tabsAbove: string[];
  @Prop() private tabsBelow: string[];
  @Prop() private initialTab: string;

  private activeTab = this.initialTab;

  private switchTab(tab) {
    this.activeTab = tab;
  }
}
</script>

<style scoped>
.container-header {
  background-color: #12263b;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  color: #fff;
}

.tab-heads {
  display: flex;
  flex-wrap: wrap;
  margin: 0;
  padding: 0;
  list-style: none;
  margin-left: 0px;
}

.tab-head {
  padding: 5px 18px;
  position: relative;
  cursor: pointer;
  font-weight: bold;
  background-color: #404040;
  color: #aaaaaa;
}

.header-above {
  padding: 8px 8px 0;
}

.header-below {
  padding: 0 8px 8px;
}

.above {
  border-top-left-radius: 8px;
  border-top-right-radius: 8px;
}

.below {
  border-bottom-left-radius: 8px;
  border-bottom-right-radius: 8px;
}

.tab-head--active {
  background-color: #1a1a1a;
  color: #dddddd;
  transition: 0.2s;
}

.container-body {
  padding: 20px 20px;
  background-color: #1a1a1a;
}
</style>
