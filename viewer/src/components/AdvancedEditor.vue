<template>
  <div>
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
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from "vue-property-decorator";
import { WebScene } from "equinox";
import CodeMirror from "codemirror";

@Component
export default class extends Vue {
  @Prop() private scene!: WebScene;

  @Prop() private onUpdateScene!: (
    json: object,
    assets: string[]
  ) => Promise<boolean>;

  mounted() {
    const editor = CodeMirror(this.$el as any, {
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

    if (result === null) {
      // bad: do something
    } else {
      const [json, assets] = result;

      if (await this.onUpdateScene(json, assets)) {
        // all good, nothing to do
      } else {
        console.error("json update failed");
      }
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
}
</script>

<style scoped></style>
