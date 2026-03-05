import "clsx";
import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { C as Card, a as Card_content, L as Label, I as Input } from "../../../chunks/label.js";
import { B as Button } from "../../../chunks/index3.js";
import { S as Switch } from "../../../chunks/switch.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let enabled = true;
    let processNoiseP = 1e-7;
    let processNoiseV = 1e-4;
    let measurementNoise = 1e-6;
    async function updateKF() {
      try {
        await invoke("update_tracker_config", { enabled, processNoiseP, processNoiseV, measurementNoise });
        appWindow.close();
      } catch (e) {
        console.error("Failed to update KF:", e);
        alert("Failed to update KF: " + e);
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div class="flex min-h-screen flex-col bg-background p-5 text-foreground"><h3 class="mb-5 text-center text-lg font-semibold">Kalman Filter</h3> `);
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_content($$renderer4, {
            class: "space-y-3 pt-4",
            children: ($$renderer5) => {
              $$renderer5.push(`<div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Enabled`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Switch($$renderer5, {
                get checked() {
                  return enabled;
                },
                set checked($$value) {
                  enabled = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Proc. Noise (P)`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: "0.0000001",
                class: "w-28 text-right",
                get value() {
                  return processNoiseP;
                },
                set value($$value) {
                  processNoiseP = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Proc. Noise (V)`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: "0.0001",
                class: "w-28 text-right",
                get value() {
                  return processNoiseV;
                },
                set value($$value) {
                  processNoiseV = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Meas. Noise`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: "0.000001",
                class: "w-28 text-right",
                get value() {
                  return measurementNoise;
                },
                set value($$value) {
                  measurementNoise = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div>`);
            },
            $$slots: { default: true }
          });
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> <div class="mt-5 flex justify-end gap-3">`);
      Button($$renderer3, {
        variant: "secondary",
        onclick: () => appWindow.close(),
        children: ($$renderer4) => {
          $$renderer4.push(`<!---->Cancel`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> `);
      Button($$renderer3, {
        onclick: updateKF,
        children: ($$renderer4) => {
          $$renderer4.push(`<!---->Apply &amp; Close`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----></div></div>`);
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
  });
}
export {
  _page as default
};
