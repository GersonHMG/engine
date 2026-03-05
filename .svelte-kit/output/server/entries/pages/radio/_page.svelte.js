import "clsx";
import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { C as Card, a as Card_content, L as Label, I as Input } from "../../../chunks/label.js";
import { B as Button } from "../../../chunks/index3.js";
import { S as Switch } from "../../../chunks/switch.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let portName = "/dev/ttyUSB0";
    let baudRate = 115200;
    let useRadio = false;
    async function updateRadio() {
      try {
        await invoke("update_radio_config", { useRadio, portName, baudRate });
        appWindow.close();
      } catch (e) {
        console.error("Failed to update radio:", e);
        alert("Failed to update radio: " + e);
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div class="flex min-h-screen flex-col bg-background p-5 text-foreground"><h3 class="mb-5 text-center text-lg font-semibold">Radio Configuration</h3> `);
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_content($$renderer4, {
            class: "space-y-3 pt-4",
            children: ($$renderer5) => {
              $$renderer5.push(`<div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Port Name`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "text",
                class: "w-40 text-right",
                get value() {
                  return portName;
                },
                set value($$value) {
                  portName = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Baud Rate`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                class: "w-28 text-right",
                get value() {
                  return baudRate;
                },
                set value($$value) {
                  baudRate = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Use Radio`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Switch($$renderer5, {
                get checked() {
                  return useRadio;
                },
                set checked($$value) {
                  useRadio = $$value;
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
        onclick: updateRadio,
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
