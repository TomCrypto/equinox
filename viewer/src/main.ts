import Vue from "vue";
import App from "@/App.vue";
import localforage from "localforage";
import Multiselect from "vue-multiselect";
import VueSlider from "vue-slider-component";
import "vue-slider-component/theme/default.css";

Vue.component("multiselect", Multiselect);
Vue.component("VueSlider", VueSlider);

Vue.config.productionTip = false;

localforage.config({
  driver: localforage.INDEXEDDB,
  name: "equinox-asset-data-v1"
});

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
})().catch(alert);
