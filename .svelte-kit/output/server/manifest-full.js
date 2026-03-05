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
			__memo(() => import('./nodes/1.js')),
			__memo(() => import('./nodes/2.js')),
			__memo(() => import('./nodes/3.js')),
			__memo(() => import('./nodes/4.js')),
			__memo(() => import('./nodes/5.js')),
			__memo(() => import('./nodes/6.js')),
			__memo(() => import('./nodes/7.js')),
			__memo(() => import('./nodes/8.js')),
			__memo(() => import('./nodes/9.js'))
		],
		remotes: {
			
		},
		routes: [
			{
				id: "/",
				pattern: /^\/$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 2 },
				endpoint: null
			},
			{
				id: "/control",
				pattern: /^\/control\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 3 },
				endpoint: null
			},
			{
				id: "/hyperparameters",
				pattern: /^\/hyperparameters\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 4 },
				endpoint: null
			},
			{
				id: "/kalman",
				pattern: /^\/kalman\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 5 },
				endpoint: null
			},
			{
				id: "/radio",
				pattern: /^\/radio\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 6 },
				endpoint: null
			},
			{
				id: "/recording",
				pattern: /^\/recording\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 7 },
				endpoint: null
			},
			{
				id: "/vision",
				pattern: /^\/vision\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 8 },
				endpoint: null
			},
			{
				id: "/wheels",
				pattern: /^\/wheels\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 9 },
				endpoint: null
			}
		],
		prerendered_routes: new Set([]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();
