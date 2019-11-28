<template>
  <div class="root">
    <ul>
      <li>
        <b>WARNING</b>: On Windows, shader builds can be extremely slow due to ANGLE; it
        is recommended to configure your browser to use native OpenGL if possible to improve
        build times; see
        <a
          class="hyperlink"
          target="_blank"
          rel="noopener noreferrer"
          href="https://github.com/mrdoob/three.js/wiki/How-to-use-OpenGL-or-ANGLE-rendering-on-Windows"
        >this link</a> for directions.
      </li>
      <li>
        <b>Note</b>: any camera interaction performed while this editor is open will be rolled back
        on any change to the scene JSON; the JSON will not update automatically.
      </li>
    </ul>
    <hr />
    <div ref="editor" class="editor" />
    <div class="log">
      <p class="error">{{ error }}</p>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene } from "equinox";
import CodeMirror from "codemirror";

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;

  @Prop() private loadAssets!: (assets: string[]) => Promise<void>;

  private error: string = "";

  mounted() {
    const editor = CodeMirror(this.$refs.editor as HTMLElement, {
      mode: "application/json",
      gutters: ["CodeMirror-lint-markers"],
      lineNumbers: true,
      tabSize: 2,
      theme: "monokai",
      lint: true
    });

    editor.on("change", () => {
      this.onJsonChange(editor.getValue());
    });

    editor.setSize(null, "100%");

    editor.setValue(JSON.stringify(this.sceneJson(), null, 2));
    editor.clearHistory();
  }

  private sceneJson(): object {
    return {
      json: this.scene.json(),
      assets: this.scene.assets()
    };
  }

  private async onJsonChange(input: string) {
    const result = this.validateJson(input);

    if (result !== null) {
      const [json, assets] = result;

      const error = await this.updateScene(json, assets);

      if (error !== null) {
        this.error = error;
      } else {
        this.error = "";
      }
    }
  }

  private validateJson(value: string): [object, string[]] | null {
    try {
      const payload = JSON.parse(value);

      if (!(payload["json"] instanceof Object)) {
        this.error = `error: json should be an object`;
        return null;
      }

      if (!(payload["assets"] instanceof Array)) {
        this.error = `error: assets should be an array`;
        return null;
      }

      for (const asset of payload["assets"]) {
        if (!(typeof asset === "string")) {
          this.error = `error: assets should be a string array`;
          return null;
        }
      }

      return [payload["json"], payload["assets"]];
    } catch {
      this.error = "JSON syntax error";
      return null;
    }
  }

  private async updateScene(
    json: object,
    assets: string[]
  ): Promise<string | null> {
    await this.loadAssets(assets);

    try {
      this.scene.set_json(json);

      for (const asset of this.scene.assets()) {
        if (!assets.includes(asset)) {
          this.scene.remove_asset(asset);
        }
      }

      return null;
    } catch (e) {
      return e.message;
    }
  }
}
</script>

<style scoped>
.root {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.editor {
  flex: 1;
}

.error {
  color: red;
  font-weight: bold;
}

.hyperlink {
  color: #abcdef;
}
</style>
