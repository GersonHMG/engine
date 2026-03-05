import { j as ensure_array_like, d as derived } from "../../../chunks/index2.js";
import { e as escape_html } from "../../../chunks/context.js";
import "@sveltejs/kit/internal";
import "../../../chunks/exports.js";
import "../../../chunks/utils.js";
import "clsx";
import "@sveltejs/kit/internal/server";
import "../../../chunks/root.js";
import "../../../chunks/state.svelte.js";
import { appWindow } from "@tauri-apps/api/window";
import { C as Card, a as Card_content, L as Label, I as Input, B as Button } from "../../../chunks/card-content.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    const allParams = [
      {
        id: "ctrl-lat-kp",
        label: "Lateral KP",
        defaultVal: 3,
        step: 0.1,
        controller: "lookahead"
      },
      {
        id: "ctrl-lat-ki",
        label: "Lateral KI",
        defaultVal: 0.1,
        step: 0.1,
        controller: "lookahead"
      },
      {
        id: "ctrl-lat-kd",
        label: "Lateral KD",
        defaultVal: 0.5,
        step: 0.1,
        controller: "lookahead"
      },
      {
        id: "ctrl-speed-kp",
        label: "Speed KP",
        defaultVal: 2,
        step: 0.1,
        controller: "lookahead"
      },
      {
        id: "ctrl-head-kp",
        label: "Heading KP",
        defaultVal: 4,
        step: 0.1,
        controller: "lookahead"
      },
      {
        id: "ctrl-target-speed",
        label: "Target Speed",
        defaultVal: 1,
        step: 0.1,
        controller: "lookahead"
      },
      {
        id: "ctrl-lookahead",
        label: "Lookahead Dist",
        defaultVal: 0.25,
        step: 0.05,
        controller: "lookahead"
      },
      {
        id: "ctrl-bb-amax",
        label: "BangBang Max Accel",
        defaultVal: 2.5,
        step: 0.1,
        controller: "bangbang"
      },
      {
        id: "ctrl-bb-vmax",
        label: "BangBang Max Vel",
        defaultVal: 5,
        step: 0.1,
        controller: "bangbang"
      },
      {
        id: "ctrl-pid-kp",
        label: "PID KP",
        defaultVal: 2,
        step: 0.1,
        controller: "pid"
      },
      {
        id: "ctrl-pid-maxv",
        label: "PID Max Vel",
        defaultVal: 1.5,
        step: 0.1,
        controller: "pid"
      }
    ];
    let controller = "lookahead";
    let values = {};
    let visibleParams = derived(() => allParams.filter((p) => p.controller === controller));
    const titles = {
      pid: "PID Settings",
      lookahead: "LookAhead PID Settings",
      bangbang: "BangBangTrajectories Settings"
    };
    let title = derived(() => titles[controller]);
    function save() {
      for (const p of allParams) {
        if (values[p.id] !== void 0) {
          localStorage.setItem(p.id, String(values[p.id]));
        }
      }
      appWindow.close();
    }
    function cancel() {
      appWindow.close();
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div class="flex min-h-screen flex-col bg-background p-5 text-foreground"><h3 class="mb-5 text-center text-lg font-semibold">${escape_html(title())}</h3> `);
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_content($$renderer4, {
            class: "space-y-3 pt-4",
            children: ($$renderer5) => {
              $$renderer5.push(`<!--[-->`);
              const each_array = ensure_array_like(visibleParams());
              for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
                let param = each_array[$$index];
                $$renderer5.push(`<div class="flex items-center justify-between">`);
                Label($$renderer5, {
                  children: ($$renderer6) => {
                    $$renderer6.push(`<!---->${escape_html(param.label)}`);
                  },
                  $$slots: { default: true }
                });
                $$renderer5.push(`<!----> `);
                Input($$renderer5, {
                  type: "number",
                  step: param.step,
                  class: "w-24 text-right",
                  get value() {
                    return values[param.id];
                  },
                  set value($$value) {
                    values[param.id] = $$value;
                    $$settled = false;
                  }
                });
                $$renderer5.push(`<!----></div>`);
              }
              $$renderer5.push(`<!--]-->`);
            },
            $$slots: { default: true }
          });
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> <div class="mt-5 flex justify-end gap-3">`);
      Button($$renderer3, {
        variant: "secondary",
        onclick: cancel,
        children: ($$renderer4) => {
          $$renderer4.push(`<!---->Cancel`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> `);
      Button($$renderer3, {
        onclick: save,
        children: ($$renderer4) => {
          $$renderer4.push(`<!---->Save &amp; Close`);
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
