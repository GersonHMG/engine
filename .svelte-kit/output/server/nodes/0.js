

export const index = 0;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/_layout.svelte.js')).default;
export const universal = {
  "prerender": true,
  "ssr": false
};
export const universal_id = "src/gui/routes/+layout.ts";
export const imports = ["_app/immutable/nodes/0.hvbzWAIR.js","_app/immutable/chunks/BtrsW6dd.js","_app/immutable/chunks/DYtdkAnv.js","_app/immutable/chunks/BPtIXCzS.js","_app/immutable/chunks/Dz2Z8ruR.js"];
export const stylesheets = ["_app/immutable/assets/0.Dt4GzbVG.css"];
export const fonts = [];
