import { e as escape_html } from "../../../chunks/context.js";
import "clsx";
import { o as onDestroy } from "../../../chunks/index-server.js";
import "@tauri-apps/api/event";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let currentVx = 0;
    let currentVy = 0;
    let currentOmega = 0;
    onDestroy(() => {
    });
    $$renderer2.push(`<div class="flex h-screen flex-col items-center overflow-hidden bg-background p-3 text-foreground"><h3 class="mb-2 text-xs font-medium uppercase tracking-widest text-muted-foreground">Omnidirectional Robot — Wheel Velocities</h3> <canvas width="340" height="340" class="rounded-md border border-border bg-[#111]"></canvas> <div class="mt-3 flex gap-4 font-mono text-xs text-muted-foreground"><span>Vx: <span class="font-bold text-foreground">${escape_html(currentVx.toFixed(2))}</span></span> <span>Vy: <span class="font-bold text-foreground">${escape_html(currentVy.toFixed(2))}</span></span> <span>ω: <span class="font-bold text-foreground">${escape_html(currentOmega.toFixed(2))}</span></span></div></div>`);
  });
}
export {
  _page as default
};
