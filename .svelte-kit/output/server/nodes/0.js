

export const index = 0;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/_layout.svelte.js')).default;
export const universal = {
  "prerender": true,
  "ssr": false
};
export const universal_id = "src/gui/routes/+layout.ts";
export const imports = ["_app/immutable/nodes/0.BpLQSfWQ.js","_app/immutable/chunks/D6eZHnRn.js","_app/immutable/chunks/CcGn78dg.js","_app/immutable/chunks/DzscFAK-.js","_app/immutable/chunks/eth6LYLS.js"];
export const stylesheets = ["_app/immutable/assets/0.BAElGqfw.css"];
export const fonts = [];
