import { b as attr_class, ac as stringify, f as derived } from "../../../chunks/index2.js";
import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { C as Card, a as Card_content, L as Label, I as Input } from "../../../chunks/label.js";
import "clsx";
import { B as Button } from "../../../chunks/index3.js";
import { e as escape_html } from "../../../chunks/context.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let filename = "record.csv";
    let status = "idle";
    let startDisabled = false;
    let stopDisabled = true;
    async function startRecording() {
      try {
        await invoke("start_recording", { filename });
        status = "recording";
        startDisabled = true;
        stopDisabled = false;
      } catch (e) {
        console.error(e);
        alert(String(e));
      }
    }
    async function stopRecording() {
      try {
        await invoke("stop_recording");
        status = "saved";
        startDisabled = false;
        stopDisabled = true;
      } catch (e) {
        console.error(e);
      }
    }
    const statusColor = derived(() => status === "recording" ? "text-green-500" : "text-muted-foreground");
    const statusText = derived(() => status === "recording" ? "Recording..." : status === "saved" ? "Saved" : "Idle");
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div class="flex min-h-screen flex-col bg-background p-5 text-foreground"><h3 class="mb-5 text-center text-lg font-semibold">Recording</h3> `);
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_content($$renderer4, {
            class: "space-y-3 pt-4",
            children: ($$renderer5) => {
              $$renderer5.push(`<div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Filename`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "text",
                class: "w-40 text-right",
                get value() {
                  return filename;
                },
                set value($$value) {
                  filename = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex gap-2">`);
              Button($$renderer5, {
                class: "flex-1",
                disabled: startDisabled,
                onclick: startRecording,
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Start`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Button($$renderer5, {
                variant: "destructive",
                class: "flex-1",
                disabled: stopDisabled,
                onclick: stopRecording,
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Stop`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----></div> <p${attr_class(`text-xs ${stringify(statusColor())}`)}>${escape_html(statusText())}</p>`);
            },
            $$slots: { default: true }
          });
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> <div class="mt-5 flex justify-end">`);
      Button($$renderer3, {
        variant: "secondary",
        onclick: () => appWindow.close(),
        children: ($$renderer4) => {
          $$renderer4.push(`<!---->Close`);
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
