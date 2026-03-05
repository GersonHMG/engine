import "clsx";
import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { C as Card, a as Card_content, L as Label, I as Input } from "../../../chunks/label.js";
import { B as Button } from "../../../chunks/index3.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let visionIp = "224.5.23.2";
    let visionPort = 10020;
    async function reconnect() {
      try {
        await invoke("update_vision_connection", { ip: visionIp, port: visionPort });
        appWindow.close();
      } catch (e) {
        console.error("Failed to update vision:", e);
        alert("Failed to reconnect: " + e);
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div class="flex min-h-screen flex-col bg-background p-5 text-foreground"><h3 class="mb-5 text-center text-lg font-semibold">Vision Connection</h3> `);
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_content($$renderer4, {
            class: "space-y-3 pt-4",
            children: ($$renderer5) => {
              $$renderer5.push(`<div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->IP`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "text",
                class: "w-40 text-right",
                get value() {
                  return visionIp;
                },
                set value($$value) {
                  visionIp = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Port`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                class: "w-28 text-right",
                get value() {
                  return visionPort;
                },
                set value($$value) {
                  visionPort = $$value;
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
        onclick: reconnect,
        children: ($$renderer4) => {
          $$renderer4.push(`<!---->Connect &amp; Close`);
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
