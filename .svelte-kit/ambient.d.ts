
// this file is generated — do not edit it


/// <reference types="@sveltejs/kit" />

/**
 * This module provides access to environment variables that are injected _statically_ into your bundle at build time and are limited to _private_ access.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Static environment variables are [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env` at build time and then statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * **_Private_ access:**
 * 
 * - This module cannot be imported into client-side code
 * - This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured)
 * 
 * For example, given the following build time environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { ENVIRONMENT, PUBLIC_BASE_URL } from '$env/static/private';
 * 
 * console.log(ENVIRONMENT); // => "production"
 * console.log(PUBLIC_BASE_URL); // => throws error during build
 * ```
 * 
 * The above values will be the same _even if_ different values for `ENVIRONMENT` or `PUBLIC_BASE_URL` are set at runtime, as they are statically replaced in your code with their build time values.
 */
declare module '$env/static/private' {
	export const ALLUSERSPROFILE: string;
	export const APPDATA: string;
	export const CARGO: string;
	export const CARGO_CFG_FEATURE: string;
	export const CARGO_CFG_PANIC: string;
	export const CARGO_CFG_TARGET_ABI: string;
	export const CARGO_CFG_TARGET_ARCH: string;
	export const CARGO_CFG_TARGET_ENDIAN: string;
	export const CARGO_CFG_TARGET_ENV: string;
	export const CARGO_CFG_TARGET_FAMILY: string;
	export const CARGO_CFG_TARGET_FEATURE: string;
	export const CARGO_CFG_TARGET_HAS_ATOMIC: string;
	export const CARGO_CFG_TARGET_OS: string;
	export const CARGO_CFG_TARGET_POINTER_WIDTH: string;
	export const CARGO_CFG_TARGET_VENDOR: string;
	export const CARGO_CFG_WINDOWS: string;
	export const CARGO_ENCODED_RUSTFLAGS: string;
	export const CARGO_HOME: string;
	export const CARGO_MAKEFLAGS: string;
	export const CARGO_MANIFEST_DIR: string;
	export const CARGO_MANIFEST_PATH: string;
	export const CARGO_PKG_AUTHORS: string;
	export const CARGO_PKG_DESCRIPTION: string;
	export const CARGO_PKG_HOMEPAGE: string;
	export const CARGO_PKG_LICENSE: string;
	export const CARGO_PKG_LICENSE_FILE: string;
	export const CARGO_PKG_NAME: string;
	export const CARGO_PKG_README: string;
	export const CARGO_PKG_REPOSITORY: string;
	export const CARGO_PKG_RUST_VERSION: string;
	export const CARGO_PKG_VERSION: string;
	export const CARGO_PKG_VERSION_MAJOR: string;
	export const CARGO_PKG_VERSION_MINOR: string;
	export const CARGO_PKG_VERSION_PATCH: string;
	export const CARGO_PKG_VERSION_PRE: string;
	export const CHROME_CRASHPAD_PIPE_NAME: string;
	export const CMAKE_PREFIX_PATH: string;
	export const COLOR: string;
	export const COLORTERM: string;
	export const CommonProgramFiles: string;
	export const CommonProgramW6432: string;
	export const COMPUTERNAME: string;
	export const ComSpec: string;
	export const CONDA_DEFAULT_ENV: string;
	export const CONDA_EXE: string;
	export const CONDA_PREFIX: string;
	export const CONDA_PROMPT_MODIFIER: string;
	export const CONDA_PYTHON_EXE: string;
	export const CONDA_SHLVL: string;
	export const CORSNIDDIR: string;
	export const DEBUG: string;
	export const DriverData: string;
	export const EDITOR: string;
	export const GEMINI_CLI_IDE_AUTH_TOKEN: string;
	export const GEMINI_CLI_IDE_SERVER_PORT: string;
	export const GEMINI_CLI_IDE_WORKSPACE_PATH: string;
	export const GENICAM_GENTL32_PATH: string;
	export const GENICAM_GENTL64_PATH: string;
	export const GIT_ASKPASS: string;
	export const HOME: string;
	export const HOMEDRIVE: string;
	export const HOMEPATH: string;
	export const HOST: string;
	export const IGCCSVC_DB: string;
	export const INIT_CWD: string;
	export const LANG: string;
	export const LD_LIBRARY_PATH: string;
	export const LOCALAPPDATA: string;
	export const LOGONSERVER: string;
	export const NODE: string;
	export const NODE_ENV: string;
	export const NODE_EXE: string;
	export const NPM_CLI_JS: string;
	export const npm_command: string;
	export const npm_config_cache: string;
	export const npm_config_globalconfig: string;
	export const npm_config_global_prefix: string;
	export const npm_config_init_module: string;
	export const npm_config_local_prefix: string;
	export const npm_config_node_gyp: string;
	export const npm_config_noproxy: string;
	export const npm_config_npm_version: string;
	export const npm_config_prefix: string;
	export const npm_config_userconfig: string;
	export const npm_config_user_agent: string;
	export const npm_execpath: string;
	export const npm_lifecycle_event: string;
	export const npm_lifecycle_script: string;
	export const npm_node_execpath: string;
	export const npm_package_json: string;
	export const NPM_PREFIX_JS: string;
	export const NPM_PREFIX_NPM_CLI_JS: string;
	export const NUMBER_OF_PROCESSORS: string;
	export const NUM_JOBS: string;
	export const OneDrive: string;
	export const OneDriveConsumer: string;
	export const OPENAI_API_KEY: string;
	export const OPT_LEVEL: string;
	export const OS: string;
	export const OUT_DIR: string;
	export const Path: string;
	export const PATHEXT: string;
	export const POWERSHELL_DISTRIBUTION_CHANNEL: string;
	export const PROCESSOR_ARCHITECTURE: string;
	export const PROCESSOR_IDENTIFIER: string;
	export const PROCESSOR_LEVEL: string;
	export const PROCESSOR_REVISION: string;
	export const PROFILE: string;
	export const ProgramData: string;
	export const ProgramFiles: string;
	export const ProgramW6432: string;
	export const PROMPT: string;
	export const PROTOC: string;
	export const PSExecutionPolicyPreference: string;
	export const PSModulePath: string;
	export const PUBLIC: string;
	export const PyCharm: string;
	export const PYTHONSTARTUP: string;
	export const PYTHON_BASIC_REPL: string;
	export const QT_ROOT: string;
	export const RUSTC: string;
	export const RUSTDOC: string;
	export const RUSTUP_HOME: string;
	export const RUSTUP_TOOLCHAIN: string;
	export const RUST_RECURSION_COUNT: string;
	export const SPINNAKER_GENTL32_CTI_VS140: string;
	export const SPINNAKER_GENTL64_CTI_VS140: string;
	export const SPINNAKER_INSTALL_PATH: string;
	export const SSL_CERT_FILE: string;
	export const SystemDrive: string;
	export const SystemRoot: string;
	export const TARGET: string;
	export const TELEDYNE_COMMON_COMPONENTS_ROOT: string;
	export const TELEDYNE_DALSA_GENICAM_ROOT: string;
	export const TELEDYNE_DALSA_GENICAM_ROOT_320: string;
	export const TEMP: string;
	export const TERM_PROGRAM: string;
	export const TERM_PROGRAM_VERSION: string;
	export const TMP: string;
	export const USERDOMAIN: string;
	export const USERDOMAIN_ROAMINGPROFILE: string;
	export const USERNAME: string;
	export const USERPROFILE: string;
	export const VSCODE_GIT_ASKPASS_EXTRA_ARGS: string;
	export const VSCODE_GIT_ASKPASS_MAIN: string;
	export const VSCODE_GIT_ASKPASS_NODE: string;
	export const VSCODE_GIT_IPC_HANDLE: string;
	export const VSCODE_INJECTION: string;
	export const VSCODE_PYTHON_AUTOACTIVATE_GUARD: string;
	export const windir: string;
	export const ZES_ENABLE_SYSMAN: string;
	export const _CONDA_EXE: string;
	export const _CONDA_ROOT: string;
	export const __CONDA_OPENSLL_CERT_FILE_SET: string;
}

/**
 * This module provides access to environment variables that are injected _statically_ into your bundle at build time and are _publicly_ accessible.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Static environment variables are [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env` at build time and then statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * **_Public_ access:**
 * 
 * - This module _can_ be imported into client-side code
 * - **Only** variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`) are included
 * 
 * For example, given the following build time environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { ENVIRONMENT, PUBLIC_BASE_URL } from '$env/static/public';
 * 
 * console.log(ENVIRONMENT); // => throws error during build
 * console.log(PUBLIC_BASE_URL); // => "http://site.com"
 * ```
 * 
 * The above values will be the same _even if_ different values for `ENVIRONMENT` or `PUBLIC_BASE_URL` are set at runtime, as they are statically replaced in your code with their build time values.
 */
declare module '$env/static/public' {
	
}

/**
 * This module provides access to environment variables set _dynamically_ at runtime and that are limited to _private_ access.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Dynamic environment variables are defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`.
 * 
 * **_Private_ access:**
 * 
 * - This module cannot be imported into client-side code
 * - This module includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured)
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 * 
 * > [!NOTE] To get correct types, environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * >
 * > ```env
 * > MY_FEATURE_FLAG=
 * > ```
 * >
 * > You can override `.env` values from the command line like so:
 * >
 * > ```sh
 * > MY_FEATURE_FLAG="enabled" npm run dev
 * > ```
 * 
 * For example, given the following runtime environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { env } from '$env/dynamic/private';
 * 
 * console.log(env.ENVIRONMENT); // => "production"
 * console.log(env.PUBLIC_BASE_URL); // => undefined
 * ```
 */
declare module '$env/dynamic/private' {
	export const env: {
		ALLUSERSPROFILE: string;
		APPDATA: string;
		CARGO: string;
		CARGO_CFG_FEATURE: string;
		CARGO_CFG_PANIC: string;
		CARGO_CFG_TARGET_ABI: string;
		CARGO_CFG_TARGET_ARCH: string;
		CARGO_CFG_TARGET_ENDIAN: string;
		CARGO_CFG_TARGET_ENV: string;
		CARGO_CFG_TARGET_FAMILY: string;
		CARGO_CFG_TARGET_FEATURE: string;
		CARGO_CFG_TARGET_HAS_ATOMIC: string;
		CARGO_CFG_TARGET_OS: string;
		CARGO_CFG_TARGET_POINTER_WIDTH: string;
		CARGO_CFG_TARGET_VENDOR: string;
		CARGO_CFG_WINDOWS: string;
		CARGO_ENCODED_RUSTFLAGS: string;
		CARGO_HOME: string;
		CARGO_MAKEFLAGS: string;
		CARGO_MANIFEST_DIR: string;
		CARGO_MANIFEST_PATH: string;
		CARGO_PKG_AUTHORS: string;
		CARGO_PKG_DESCRIPTION: string;
		CARGO_PKG_HOMEPAGE: string;
		CARGO_PKG_LICENSE: string;
		CARGO_PKG_LICENSE_FILE: string;
		CARGO_PKG_NAME: string;
		CARGO_PKG_README: string;
		CARGO_PKG_REPOSITORY: string;
		CARGO_PKG_RUST_VERSION: string;
		CARGO_PKG_VERSION: string;
		CARGO_PKG_VERSION_MAJOR: string;
		CARGO_PKG_VERSION_MINOR: string;
		CARGO_PKG_VERSION_PATCH: string;
		CARGO_PKG_VERSION_PRE: string;
		CHROME_CRASHPAD_PIPE_NAME: string;
		CMAKE_PREFIX_PATH: string;
		COLOR: string;
		COLORTERM: string;
		CommonProgramFiles: string;
		CommonProgramW6432: string;
		COMPUTERNAME: string;
		ComSpec: string;
		CONDA_DEFAULT_ENV: string;
		CONDA_EXE: string;
		CONDA_PREFIX: string;
		CONDA_PROMPT_MODIFIER: string;
		CONDA_PYTHON_EXE: string;
		CONDA_SHLVL: string;
		CORSNIDDIR: string;
		DEBUG: string;
		DriverData: string;
		EDITOR: string;
		GEMINI_CLI_IDE_AUTH_TOKEN: string;
		GEMINI_CLI_IDE_SERVER_PORT: string;
		GEMINI_CLI_IDE_WORKSPACE_PATH: string;
		GENICAM_GENTL32_PATH: string;
		GENICAM_GENTL64_PATH: string;
		GIT_ASKPASS: string;
		HOME: string;
		HOMEDRIVE: string;
		HOMEPATH: string;
		HOST: string;
		IGCCSVC_DB: string;
		INIT_CWD: string;
		LANG: string;
		LD_LIBRARY_PATH: string;
		LOCALAPPDATA: string;
		LOGONSERVER: string;
		NODE: string;
		NODE_ENV: string;
		NODE_EXE: string;
		NPM_CLI_JS: string;
		npm_command: string;
		npm_config_cache: string;
		npm_config_globalconfig: string;
		npm_config_global_prefix: string;
		npm_config_init_module: string;
		npm_config_local_prefix: string;
		npm_config_node_gyp: string;
		npm_config_noproxy: string;
		npm_config_npm_version: string;
		npm_config_prefix: string;
		npm_config_userconfig: string;
		npm_config_user_agent: string;
		npm_execpath: string;
		npm_lifecycle_event: string;
		npm_lifecycle_script: string;
		npm_node_execpath: string;
		npm_package_json: string;
		NPM_PREFIX_JS: string;
		NPM_PREFIX_NPM_CLI_JS: string;
		NUMBER_OF_PROCESSORS: string;
		NUM_JOBS: string;
		OneDrive: string;
		OneDriveConsumer: string;
		OPENAI_API_KEY: string;
		OPT_LEVEL: string;
		OS: string;
		OUT_DIR: string;
		Path: string;
		PATHEXT: string;
		POWERSHELL_DISTRIBUTION_CHANNEL: string;
		PROCESSOR_ARCHITECTURE: string;
		PROCESSOR_IDENTIFIER: string;
		PROCESSOR_LEVEL: string;
		PROCESSOR_REVISION: string;
		PROFILE: string;
		ProgramData: string;
		ProgramFiles: string;
		ProgramW6432: string;
		PROMPT: string;
		PROTOC: string;
		PSExecutionPolicyPreference: string;
		PSModulePath: string;
		PUBLIC: string;
		PyCharm: string;
		PYTHONSTARTUP: string;
		PYTHON_BASIC_REPL: string;
		QT_ROOT: string;
		RUSTC: string;
		RUSTDOC: string;
		RUSTUP_HOME: string;
		RUSTUP_TOOLCHAIN: string;
		RUST_RECURSION_COUNT: string;
		SPINNAKER_GENTL32_CTI_VS140: string;
		SPINNAKER_GENTL64_CTI_VS140: string;
		SPINNAKER_INSTALL_PATH: string;
		SSL_CERT_FILE: string;
		SystemDrive: string;
		SystemRoot: string;
		TARGET: string;
		TELEDYNE_COMMON_COMPONENTS_ROOT: string;
		TELEDYNE_DALSA_GENICAM_ROOT: string;
		TELEDYNE_DALSA_GENICAM_ROOT_320: string;
		TEMP: string;
		TERM_PROGRAM: string;
		TERM_PROGRAM_VERSION: string;
		TMP: string;
		USERDOMAIN: string;
		USERDOMAIN_ROAMINGPROFILE: string;
		USERNAME: string;
		USERPROFILE: string;
		VSCODE_GIT_ASKPASS_EXTRA_ARGS: string;
		VSCODE_GIT_ASKPASS_MAIN: string;
		VSCODE_GIT_ASKPASS_NODE: string;
		VSCODE_GIT_IPC_HANDLE: string;
		VSCODE_INJECTION: string;
		VSCODE_PYTHON_AUTOACTIVATE_GUARD: string;
		windir: string;
		ZES_ENABLE_SYSMAN: string;
		_CONDA_EXE: string;
		_CONDA_ROOT: string;
		__CONDA_OPENSLL_CERT_FILE_SET: string;
		[key: `PUBLIC_${string}`]: undefined;
		[key: `${string}`]: string | undefined;
	}
}

/**
 * This module provides access to environment variables set _dynamically_ at runtime and that are _publicly_ accessible.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Dynamic environment variables are defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`.
 * 
 * **_Public_ access:**
 * 
 * - This module _can_ be imported into client-side code
 * - **Only** variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`) are included
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 * 
 * > [!NOTE] To get correct types, environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * >
 * > ```env
 * > MY_FEATURE_FLAG=
 * > ```
 * >
 * > You can override `.env` values from the command line like so:
 * >
 * > ```sh
 * > MY_FEATURE_FLAG="enabled" npm run dev
 * > ```
 * 
 * For example, given the following runtime environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://example.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { env } from '$env/dynamic/public';
 * console.log(env.ENVIRONMENT); // => undefined, not public
 * console.log(env.PUBLIC_BASE_URL); // => "http://example.com"
 * ```
 * 
 * ```
 * 
 * ```
 */
declare module '$env/dynamic/public' {
	export const env: {
		[key: `PUBLIC_${string}`]: string | undefined;
	}
}
