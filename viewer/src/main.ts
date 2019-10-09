import Vue from "vue";
import App from "./App.vue";
import localforage from "localforage";
import { isMobile } from "mobile-device-detect";

Vue.config.productionTip = false;

localforage.setDriver(localforage.INDEXEDDB);

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
})().catch(err => {
  // mobile devices do not have any dev tools
  isMobile ? alert(err) : console.error(err);
});
