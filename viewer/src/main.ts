import Vue from "vue";
import App from "@/App.vue";
import Multiselect from "vue-multiselect";
import VueSlider from "vue-slider-component";
import "vue-slider-component/theme/default.css";
import "codemirror";
import "codemirror/lib/codemirror.css";
import "codemirror/theme/monokai.css";
import "codemirror/addon/lint/lint.css";
import "codemirror/addon/lint/lint";
import "codemirror/addon/lint/json-lint";
import "codemirror/mode/javascript/javascript";
import { library } from "@fortawesome/fontawesome-svg-core";
import { faHdd, faTrash } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/vue-fontawesome";

library.add(faHdd, faTrash);

(window as any).jsonlint = require("jsonlint-mod");

Vue.component("multiselect", Multiselect);
Vue.component("VueSlider", VueSlider);
Vue.component("font-awesome-icon", FontAwesomeIcon);

Vue.config.productionTip = false;

(async () => {
  ((equinox: typeof import("equinox")) => {
    console.info("WASM module loaded: " + equinox.version());
    equinox.initialize_logging(); // called once on startup

    new Vue({
      render: h =>
        h(App, {
          props: { equinox }
        })
    }).$mount("#app");
  })(await import("equinox"));
})().catch(alert);
