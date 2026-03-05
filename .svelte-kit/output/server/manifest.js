export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set([".gitkeep"]),
	mimeTypes: {},
	_: {
		client: {start:"_app/immutable/entry/start.wlzUKxCU.js",app:"_app/immutable/entry/app.BYXdwTlq.js",imports:["_app/immutable/entry/start.wlzUKxCU.js","_app/immutable/chunks/BxZt6sdU.js","_app/immutable/chunks/CcGn78dg.js","_app/immutable/chunks/BA5-3jja.js","_app/immutable/chunks/dQk9N1Wb.js","_app/immutable/chunks/Bo-nF3b5.js","_app/immutable/entry/app.BYXdwTlq.js","_app/immutable/chunks/CcGn78dg.js","_app/immutable/chunks/D1MBF5Hc.js","_app/immutable/chunks/DNslKuZY.js","_app/immutable/chunks/D6eZHnRn.js","_app/immutable/chunks/BMU8zQGq.js","_app/immutable/chunks/Bo-nF3b5.js","_app/immutable/chunks/Pc97PY2K.js","_app/immutable/chunks/eth6LYLS.js","_app/immutable/chunks/62RbDeLv.js","_app/immutable/chunks/BA5-3jja.js","_app/immutable/chunks/z5ijKnRf.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js'))
		],
		remotes: {
			
		},
		routes: [
			
		],
		prerendered_routes: new Set(["/","/control","/hyperparameters","/kalman","/radio","/recording","/vision","/wheels"]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();
