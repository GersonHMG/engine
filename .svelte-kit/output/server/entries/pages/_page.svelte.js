import "clsx";
import { o as onDestroy } from "../../chunks/index-server.js";
import "@tauri-apps/api/event";
import "@tauri-apps/api/window";
import { p as pathDrawMode, a as pathPoints, s as scriptPath, b as scriptStatus, c as pathTraceMode, v as visualizeVelocities, d as controlRobotId, e as controlTeam } from "../../chunks/app.js";
import { invoke } from "@tauri-apps/api/tauri";
import { e as escape_html } from "../../chunks/context.js";
import { g as get } from "../../chunks/index.js";
import { s as store_get, a as attr, u as unsubscribe_stores } from "../../chunks/index2.js";
import { open } from "@tauri-apps/api/dialog";
import { B as Button } from "../../chunks/index3.js";
function screenToField(vp, clientX, clientY, canvasRect) {
  const cx = vp.width / 2 + vp.panX;
  const cy = vp.height / 2 + vp.panY;
  const mouseX = clientX - canvasRect.left;
  const mouseY = clientY - canvasRect.top;
  return {
    x: (mouseX - cx) / vp.scale / 1e3,
    y: -(mouseY - cy) / vp.scale / 1e3
  };
}
function FieldCanvas($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let canvas;
    let mouseCoords = "0, 0";
    let scale = 0.08;
    let panX = 0;
    let panY = 0;
    let isDragging = false;
    let didDrag = false;
    let lastMouseX = 0;
    let lastMouseY = 0;
    function getViewport() {
      return {
        width: canvas.width,
        height: canvas.height,
        panX,
        panY,
        scale
      };
    }
    function resize() {
      return;
    }
    function updateMouseCoords(clientX, clientY) {
      return;
    }
    function onMouseMove(e) {
      updateMouseCoords(e.clientX, e.clientY);
      if (!isDragging) return;
      const dx = e.clientX - lastMouseX;
      const dy = e.clientY - lastMouseY;
      if (Math.abs(dx) > 2 || Math.abs(dy) > 2) {
        didDrag = true;
      }
      panX += dx;
      panY += dy;
      lastMouseX = e.clientX;
      lastMouseY = e.clientY;
    }
    function onMouseUp(e) {
      if (isDragging && !didDrag && e.target === canvas) {
        const vp = getViewport();
        const rect = canvas.getBoundingClientRect();
        const pos = screenToField(vp, e.clientX, e.clientY, rect);
        if (get(pathDrawMode)) {
          pathPoints.update((pts) => [...pts, pos]);
        }
      }
      isDragging = false;
    }
    onDestroy(() => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      window.removeEventListener("resize", resize);
    });
    $$renderer2.push(`<div class="relative flex-1 overflow-hidden bg-[#111]"><canvas class="h-full w-full" style="background-color: #A9A9A9;"></canvas> <div class="pointer-events-none absolute left-4 top-4 rounded bg-black/50 px-2.5 py-1 font-mono text-sm text-white">${escape_html(mouseCoords)}</div> <div class="absolute bottom-4 right-4 flex gap-1.5"><button class="flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-lg text-white hover:bg-white/20">-</button> <button class="flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-lg text-white hover:bg-white/20">+</button></div></div>`);
  });
}
function FieldToolbar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    onDestroy(() => {
    });
    async function selectScript() {
      const selected = await open({
        filters: [{ name: "Lua Scripts", extensions: ["lua"] }],
        multiple: false
      });
      if (selected && typeof selected === "string") {
        scriptPath.set(selected);
        try {
          await invoke("load_script", { path: selected });
          scriptStatus.set("loaded");
        } catch (e) {
          console.error("Failed to load script:", e);
          scriptStatus.set("error");
        }
      }
    }
    async function playScript() {
      try {
        await invoke("resume_script");
        scriptStatus.set("running");
      } catch (e) {
        console.error("Failed to resume script:", e);
      }
    }
    async function stopScript() {
      try {
        await invoke("pause_script");
        scriptStatus.set("paused");
      } catch (e) {
        console.error("Failed to pause script:", e);
      }
    }
    function filename(path) {
      if (!path) return "No script";
      const parts = path.replace(/\\/g, "/").split("/");
      return parts[parts.length - 1];
    }
    $$renderer2.push(`<div class="flex h-9 shrink-0 items-center gap-2 border-b border-border bg-card px-3">`);
    Button($$renderer2, {
      variant: "ghost",
      size: "sm",
      class: "h-7 w-7 p-0 text-muted-foreground hover:text-foreground",
      onclick: selectScript,
      title: "Load Lua Script",
      children: ($$renderer3) => {
        $$renderer3.push(`<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"></path><path d="M14 2v4a2 2 0 0 0 2 2h4"></path></svg>`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----> `);
    Button($$renderer2, {
      variant: "ghost",
      size: "sm",
      class: "h-7 w-7 p-0 text-muted-foreground hover:text-green-500",
      onclick: playScript,
      disabled: !store_get($$store_subs ??= {}, "$scriptPath", scriptPath),
      title: "Play Script",
      children: ($$renderer3) => {
        $$renderer3.push(`<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----> `);
    Button($$renderer2, {
      variant: "ghost",
      size: "sm",
      class: "h-7 w-7 p-0 text-muted-foreground hover:text-red-500",
      onclick: stopScript,
      disabled: !store_get($$store_subs ??= {}, "$scriptPath", scriptPath),
      title: "Stop Script",
      children: ($$renderer3) => {
        $$renderer3.push(`<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><rect x="4" y="4" width="16" height="16" rx="2"></rect></svg>`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----> <div class="h-4 w-px bg-border"></div> <span class="max-w-[200px] truncate text-xs text-muted-foreground"${attr("title", store_get($$store_subs ??= {}, "$scriptPath", scriptPath))}>${escape_html(filename(store_get($$store_subs ??= {}, "$scriptPath", scriptPath)))}</span> <div class="flex-1"></div> <canvas width="120" height="24" class="rounded border border-border bg-[#222]"></canvas></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function BottomPanel($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let capturing = false;
    function toggleTrace() {
      pathTraceMode.update((v) => !v);
    }
    function toggleVectors() {
      visualizeVelocities.update((v) => !v);
    }
    function toggleCapture() {
      capturing = !capturing;
    }
    onDestroy(() => {
    });
    $$renderer2.push(`<div class="flex h-44 shrink-0 items-stretch gap-2 border-t border-border bg-card px-3 py-2"><div class="flex w-24 shrink-0 flex-col justify-center gap-1.5"><span class="text-center text-[10px] font-semibold text-muted-foreground">Robot ${escape_html(store_get($$store_subs ??= {}, "$controlRobotId", controlRobotId))} · ${escape_html(store_get($$store_subs ??= {}, "$controlTeam", controlTeam) === 0 ? "Blue" : "Yellow")}</span> `);
    Button($$renderer2, {
      variant: capturing ? "default" : "secondary",
      size: "sm",
      class: "h-7 text-[10px]",
      onclick: toggleCapture,
      children: ($$renderer3) => {
        $$renderer3.push(`<!---->${escape_html(capturing ? "Capture ON" : "Capture OFF")}`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----> `);
    Button($$renderer2, {
      variant: "secondary",
      size: "sm",
      class: "h-7 text-[10px]",
      onclick: toggleTrace,
      children: ($$renderer3) => {
        $$renderer3.push(`<!---->${escape_html("Trace OFF")}`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----> `);
    Button($$renderer2, {
      variant: "secondary",
      size: "sm",
      class: "h-7 text-[10px]",
      onclick: toggleVectors,
      children: ($$renderer3) => {
        $$renderer3.push(`<!---->${escape_html("Vectors OFF")}`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----></div> <div class="w-px shrink-0 bg-border"></div> <div class="flex min-w-0 flex-1 gap-1"><div class="flex flex-1 flex-col gap-0.5"><div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-red-400">Vx</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div> <div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-orange-400">X</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div></div> <div class="flex flex-1 flex-col gap-0.5"><div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-green-400">Vy</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div> <div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-cyan-400">Y</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div></div> <div class="flex flex-1 flex-col gap-0.5"><div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-blue-400">ω</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div> <div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-purple-400">θ</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div></div></div></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    onDestroy(() => {
    });
    $$renderer2.push(`<div class="flex h-screen flex-col overflow-hidden rounded-lg border border-border bg-background text-foreground"><div class="flex h-8 shrink-0 items-center justify-between bg-card" data-tauri-drag-region=""><span class="pointer-events-none select-none pl-3 text-xs font-medium text-muted-foreground" data-tauri-drag-region="">Sysmic Engine</span> <div class="flex h-full"><button class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-muted" aria-label="Minimize"><svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"></rect></svg></button> <button class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-muted" aria-label="Maximize"><svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9"></rect></svg></button> <button class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-red-600 hover:text-white" aria-label="Close"><svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" stroke-width="1.2"><line x1="0" y1="0" x2="10" y2="10"></line><line x1="10" y1="0" x2="0" y2="10"></line></svg></button></div></div> <div class="flex flex-1 overflow-hidden"><aside class="flex shrink-0 flex-col gap-2 overflow-y-auto border-r border-border bg-card p-2"><button class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground" title="Vision Connection"><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"></path><circle cx="12" cy="12" r="3"></circle></svg></button> <button class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground" title="Radio Configuration"><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"></path><path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.4"></path><circle cx="12" cy="12" r="2"></circle><path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.4"></path><path d="M19.1 4.9C23 8.8 23 15.1 19.1 19"></path></svg></button> <button class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground" title="Kalman Filter"><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v18h18"></path><path d="m19 9-5 5-4-4-3 3"></path></svg></button> <button class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground" title="Recording"><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><circle cx="12" cy="12" r="4" fill="currentColor"></circle></svg></button> <button class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground" title="Manual Control"><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="6" width="20" height="12" rx="2"></rect><circle cx="12" cy="12" r="2"></circle><path d="M6 12h.01"></path><path d="M18 12h.01"></path></svg></button></aside> <div class="flex flex-1 flex-col overflow-hidden">`);
    FieldToolbar($$renderer2);
    $$renderer2.push(`<!----> `);
    FieldCanvas($$renderer2);
    $$renderer2.push(`<!----></div></div> `);
    BottomPanel($$renderer2);
    $$renderer2.push(`<!----></div>`);
  });
}
export {
  _page as default
};
