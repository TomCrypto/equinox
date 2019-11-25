<template>
  <div class="root">
    <p>
      Obtain full control by directly editing the scene's underlying representation. Note that
      some changes (especially changing the geometry modifier stack and changing non-symbolic
      parameters) may trigger shader rebuilds which can take a few seconds.
    </p>
    <p>
      On Windows, shader builds can be very slow due to the ANGLE GLSL to HLSL conversion. It
      is recommended to switch to native OpenGL if possible.
    </p>
    <p>
      Note: any camera interaction performed while this editor is open will be rolled back
      on any change to the scene JSON; in other words, the JSON does not update by itself.
    </p>
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
import SceneSchema from "../helpers/scene_schema";
import ajv from "ajv";

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;

  @Prop() private onUpdateScene!: (
    json: object,
    assets: string[]
  ) => Promise<string | null>;

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

      const error = await this.onUpdateScene(json, assets);

      if (error !== null) {
        this.error = `renderer error: ${error}`;
      } else {
        this.error = "";
      }
    }
  }

  private validateJson(value: string): [object, string[]] | null {
    try {
      const payload = JSON.parse(value);

      const validator = ajv();

      if (!validator.validate(SceneSchema, payload)) {
        const error = validator.errors[0];

        this.error = `error: scene.${error.dataPath}: ${error.message}`;

        console.log(validator.errors);
        return null;
      } else {
        return [payload["json"], payload["assets"]];
      }

      /*if (!(payload["json"] instanceof Object)) {
        return null;
      }

      if (!(payload["assets"] instanceof Array)) {
        return null;
      }

      for (const asset of payload["assets"]) {
        if (!(typeof asset === "string")) {
          return null;
        }
      }*/
    } catch {
      this.error = "JSON syntax error";
      return null;
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
</style>
