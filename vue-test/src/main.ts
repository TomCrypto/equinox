import Vue from "vue";
import App from "./App.vue";

Vue.config.productionTip = false;

(async () => {
  ((equinox: typeof import("equinox")) => {
    console.log("WASM module loaded: " + equinox.version());
    equinox.initialize_logging(); // called once on startup

    new Vue({
      render: h =>
        h(App, {
          props: { equinox }
        })
    }).$mount("#app");
  })(await import("equinox"));
})().catch(console.error);
