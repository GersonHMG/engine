

export const index = 0;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/_layout.svelte.js')).default;
export const universal = {
  "prerender": true,
  "ssr": false
};
export const universal_id = "src/gui/routes/+layout.ts";
export const imports = ["_app/immutable/nodes/0.DoUsyW3D.js","_app/immutable/chunks/CQIohLAQ.js","_app/immutable/chunks/DzyAtdHu.js","_app/immutable/chunks/DdtLLkvc.js","_app/immutable/chunks/B9yMtkw2.js"];
export const stylesheets = ["_app/immutable/assets/0.BF5-5_gG.css"];
export const fonts = [];
