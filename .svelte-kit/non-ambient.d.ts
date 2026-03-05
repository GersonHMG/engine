
// this file is generated — do not edit it


declare module "svelte/elements" {
	export interface HTMLAttributes<T> {
		'data-sveltekit-keepfocus'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-noscroll'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-preload-code'?:
			| true
			| ''
			| 'eager'
			| 'viewport'
			| 'hover'
			| 'tap'
			| 'off'
			| undefined
			| null;
		'data-sveltekit-preload-data'?: true | '' | 'hover' | 'tap' | 'off' | undefined | null;
		'data-sveltekit-reload'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-replacestate'?: true | '' | 'off' | undefined | null;
	}
}

export {};


declare module "$app/types" {
	export interface AppTypes {
		RouteId(): "/" | "/control" | "/hyperparameters" | "/kalman" | "/radio" | "/recording" | "/vision" | "/wheels";
		RouteParams(): {
			
		};
		LayoutParams(): {
			"/": Record<string, never>;
			"/control": Record<string, never>;
			"/hyperparameters": Record<string, never>;
			"/kalman": Record<string, never>;
			"/radio": Record<string, never>;
			"/recording": Record<string, never>;
			"/vision": Record<string, never>;
			"/wheels": Record<string, never>
		};
		Pathname(): "/" | "/control" | "/hyperparameters" | "/kalman" | "/radio" | "/recording" | "/vision" | "/wheels";
		ResolvedPathname(): `${"" | `/${string}`}${ReturnType<AppTypes['Pathname']>}`;
		Asset(): "/.gitkeep" | string & {};
	}
}