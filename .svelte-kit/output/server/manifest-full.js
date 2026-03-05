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
		client: {start:"_app/immutable/entry/start.Drmzf0AE.js",app:"_app/immutable/entry/app.C2jGo8tS.js",imports:["_app/immutable/entry/start.Drmzf0AE.js","_app/immutable/chunks/dDuf4WKq.js","_app/immutable/chunks/DzyAtdHu.js","_app/immutable/chunks/CRIQiTCI.js","_app/immutable/chunks/CIX7PglF.js","_app/immutable/chunks/8zYVaBGU.js","_app/immutable/entry/app.C2jGo8tS.js","_app/immutable/chunks/DzyAtdHu.js","_app/immutable/chunks/CUnQrKgh.js","_app/immutable/chunks/CQIohLAQ.js","_app/immutable/chunks/8zYVaBGU.js","_app/immutable/chunks/CafT1VPP.js","_app/immutable/chunks/B9yMtkw2.js","_app/immutable/chunks/BQnOT0dP.js","_app/immutable/chunks/CRIQiTCI.js","_app/immutable/chunks/B421oLyl.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
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
