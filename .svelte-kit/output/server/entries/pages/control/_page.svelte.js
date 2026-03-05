import { b as attr_class, s as store_get, u as unsubscribe_stores } from "../../../chunks/index2.js";
import "@tauri-apps/api/window";
import "@tauri-apps/api/event";
import { C as Card, a as Card_content, L as Label, I as Input } from "../../../chunks/label.js";
import "clsx";
import { S as Switch } from "../../../chunks/switch.js";
import { g as gamepadConnected, f as gamepadStatus } from "../../../chunks/app.js";
import { e as escape_html } from "../../../chunks/context.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let mode = "xbox";
    let active = false;
    let team = 0;
    let robotId = 0;
    let scaleVx = 1;
    let scaleVy = 1;
    let scaleW = 1;
    let visVels = false;
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div class="flex min-h-screen flex-col bg-background p-5 text-foreground"><h3 class="mb-5 text-center text-lg font-semibold">Manual Control</h3> `);
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_content($$renderer4, {
            class: "space-y-3 pt-4",
            children: ($$renderer5) => {
              $$renderer5.push(`<div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Mode`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              $$renderer5.select(
                {
                  value: mode,
                  class: "h-8 rounded-md border border-input bg-background px-2 text-sm"
                },
                ($$renderer6) => {
                  $$renderer6.option({ value: "xbox" }, ($$renderer7) => {
                    $$renderer7.push(`Xbox`);
                  });
                  $$renderer6.option({ value: "keyboard" }, ($$renderer7) => {
                    $$renderer7.push(`Keyboard`);
                  });
                }
              );
              $$renderer5.push(`</div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Active`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Switch($$renderer5, {
                get checked() {
                  return active;
                },
                set checked($$value) {
                  active = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Team`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              $$renderer5.select(
                {
                  value: team,
                  class: "h-8 rounded-md border border-input bg-background px-2 text-sm"
                },
                ($$renderer6) => {
                  $$renderer6.option({ value: 0 }, ($$renderer7) => {
                    $$renderer7.push(`Blue`);
                  });
                  $$renderer6.option({ value: 1 }, ($$renderer7) => {
                    $$renderer7.push(`Yellow`);
                  });
                }
              );
              $$renderer5.push(`</div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Robot ID`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                min: 0,
                max: 15,
                class: "w-16 text-right",
                get value() {
                  return robotId;
                },
                set value($$value) {
                  robotId = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Scale Vx`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: 0.1,
                class: "w-16 text-right",
                get value() {
                  return scaleVx;
                },
                set value($$value) {
                  scaleVx = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Scale Vy`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: 0.1,
                class: "w-16 text-right",
                get value() {
                  return scaleVy;
                },
                set value($$value) {
                  scaleVy = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Scale ω`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: 0.1,
                class: "w-16 text-right",
                get value() {
                  return scaleW;
                },
                set value($$value) {
                  scaleW = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Vis. Velocities`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Switch($$renderer5, {
                get checked() {
                  return visVels;
                },
                set checked($$value) {
                  visVels = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <p${attr_class("text-xs", void 0, {
                "text-green-500": store_get($$store_subs ??= {}, "$gamepadConnected", gamepadConnected),
                "text-muted-foreground": !store_get($$store_subs ??= {}, "$gamepadConnected", gamepadConnected)
              })}>${escape_html(store_get($$store_subs ??= {}, "$gamepadStatus", gamepadStatus))}</p>`);
            },
            $$slots: { default: true }
          });
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> <div class="mt-5 flex justify-end"><button class="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90">Close</button></div></div>`);
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
