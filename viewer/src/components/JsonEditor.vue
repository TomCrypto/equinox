<template>
  <div class="editor">
    <textarea
      class="textbox"
      :value="json"
      v-on:input="onJsonChange($event.target)"
      v-on:keydown="onKeyDown($event)"
    />
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene } from "equinox";

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;
  @Prop() private onUpdateScene!: (
    json: object,
    assets: string[]
  ) => Promise<boolean>;
  @Prop() private onClose!: () => void;

  private json!: string;

  private onKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      this.onClose();
      event.preventDefault();
    }
  }

  private validateJson(value: string): [object, string[]] | null {
    try {
      const payload = JSON.parse(value);

      if (!(payload["json"] instanceof Object)) {
        return null;
      }

      if (!(payload["assets"] instanceof Array)) {
        return null;
      }

      for (const asset of payload["assets"]) {
        if (!(typeof asset === "string")) {
          return null;
        }
      }

      return [payload["json"], payload["assets"]];
    } catch {
      return null;
    }
  }

  private async onJsonChange(target: HTMLTextAreaElement) {
    const result = this.validateJson(target.value);

    if (result === null) {
      target.classList.add("invalid");
    } else {
      const [json, assets] = result;

      if (await this.onUpdateScene(json, assets)) {
        target.classList.remove("invalid");
      } else {
        target.classList.add("invalid");
      }
    }
  }

  created() {
    const payload = {
      json: this.scene.json(),
      assets: this.scene.assets()
    };

    this.json = JSON.stringify(payload, null, 2);
  }
}
</script>

<style scoped>
.editor {
  position: absolute;

  top: 50%;
  left: 50%;

  width: 95%;
  height: 90%;

  transform: translate(-50%, -50%);
  border: 2px solid black;
  border-radius: 5px;
}

.editor .textbox {
  width: 100%;
  height: 100%;

  box-sizing: border-box;

  padding: 0 6px;
  border: none;
  opacity: 0.85;
  resize: none;

  font-size: 0.8em;
  line-height: normal;
  font-family: monospace;
  font-weight: normal;

  background-color: black;
  color: white;
}

.editor .invalid {
  background-color: rgb(32, 0, 0);
}

.editor .textbox:focus {
  border: none;
  outline: none;
}
</style>
