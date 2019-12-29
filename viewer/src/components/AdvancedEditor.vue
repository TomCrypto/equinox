<template>
  <div class="root">
    <ul>
      <li>
        <b>WARNING</b>: On Windows, shader builds can be extremely slow due to
        ANGLE; it is recommended to configure your browser to use native OpenGL
        if possible to improve build times; see
        <a
          class="hyperlink"
          target="_blank"
          rel="noopener noreferrer"
          href="https://github.com/mrdoob/three.js/wiki/How-to-use-OpenGL-or-ANGLE-rendering-on-Windows"
        >this link</a>
        for directions.
      </li>
      <li>
        <b>Note</b>: any camera interaction performed while this editor is open
        will be rolled back on any change to the scene JSON; the JSON will not
        update automatically.
      </li>
    </ul>
    <hr />
    <textarea ref="editor" />
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

  private error: string = "";

  mounted() {
    const editor = CodeMirror.fromTextArea(
      this.$refs.editor as HTMLTextAreaElement,
      {
        mode: "application/json",
        gutters: ["CodeMirror-lint-markers"],
        lineNumbers: true,
        tabSize: 2,
        theme: "monokai",
        lint: true
      }
    );

    editor.on("change", () => {
      this.onJsonChange(editor.getValue());
    });

    editor.setValue(JSON.stringify(this.scene.json(), null, 2));
    editor.clearHistory();
  }

  private async onJsonChange(input: string) {
    const json = this.validateJson(input);

    if (json !== null) {
      const error = this.updateScene(json);

      if (error !== null) {
        this.error = error;
      } else {
        this.error = "";
      }
    }
  }

  private validateJson(value: string): any | null {
    try {
      return JSON.parse(value);
    } catch {
      this.error = "JSON syntax error";
      return null;
    }
  }

  private updateScene(json: object): string | null {
    try {
      this.scene.set_json(json);

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

.root hr {
  width: calc(100% - 8px);
}

.error {
  color: red;
  font-weight: bold;
}

.hyperlink {
  color: #abcdef;
}
</style>

<style>
.CodeMirror {
  flex: 1;
}
</style>
