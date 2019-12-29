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
      <slot :name="`tab-panel-${this.activeTab}`"></slot>
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
  @Prop() private tabsAbove!: string[];
  @Prop() private tabsBelow!: string[];
  @Prop() private defaultTab!: string;

  private activeTab = this.defaultTab;

  private switchTab(tab: string) {
    this.activeTab = tab;
  }
}
</script>

<style scoped>
.container {
  display: flex;
  flex-direction: column;
  background-color: #1a1a1a;
}

.container-header {
  background-color: #12263b;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  color: #dddddd;
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
  margin-right: 4px;
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

.tab-head:hover {
  background-color: #343434;
  transition: 0.2s;
}

.tab-head--active {
  background-color: #1a1a1a;
  color: #dddddd;
  transition: 0.2s;
}

.tab-head--active:hover {
  background-color: #1a1a1a;
  color: #dddddd;
}

.container-body {
  margin: 10px 20px;
  background-color: #1a1a1a;
  color: #dddddd;
  flex: 1;
  overflow-y: auto;
  user-select: text;
}
</style>
