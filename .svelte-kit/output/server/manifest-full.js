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
		client: {start:"_app/immutable/entry/start.CXVgWsil.js",app:"_app/immutable/entry/app.zvxoHANA.js",imports:["_app/immutable/entry/start.CXVgWsil.js","_app/immutable/chunks/Bq-VEB-M.js","_app/immutable/chunks/DYtdkAnv.js","_app/immutable/chunks/BLH0v92E.js","_app/immutable/chunks/BVulNh5o.js","_app/immutable/chunks/r66zEmrs.js","_app/immutable/entry/app.zvxoHANA.js","_app/immutable/chunks/DYtdkAnv.js","_app/immutable/chunks/f56KndK6.js","_app/immutable/chunks/BtrsW6dd.js","_app/immutable/chunks/r66zEmrs.js","_app/immutable/chunks/BsU15W_X.js","_app/immutable/chunks/Dz2Z8ruR.js","_app/immutable/chunks/BAcSjS7l.js","_app/immutable/chunks/BLH0v92E.js","_app/immutable/chunks/ClFKHiMQ.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js')),
			__memo(() => import('./nodes/2.js')),
			__memo(() => import('./nodes/3.js')),
			__memo(() => import('./nodes/4.js'))
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
				id: "/hyperparameters",
				pattern: /^\/hyperparameters\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 3 },
				endpoint: null
			},
			{
				id: "/wheels",
				pattern: /^\/wheels\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 4 },
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
