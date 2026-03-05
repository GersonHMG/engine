import { writable } from 'svelte/store';

// --- Types ---
export interface Robot {
	id: number;
	x: number;
	y: number;
	theta: number;
	vx?: number;
	vy?: number;
	cmd_vx?: number;
	cmd_vy?: number;
	cmd_angular?: number;
}

// --- Navigation ---
export const activeTab = writable<'connection' | 'control' | 'script'>('connection');

// --- Vision ---
export const robotsBlue = writable<Robot[]>([]);
export const robotsYellow = writable<Robot[]>([]);
export const ball = writable({ x: 0, y: 0 });
export const currentPPS = writable(0);
export const ppsHistory = writable<number[]>(new Array(50).fill(0));
export const lastVisionUpdate = writable(0);

// --- Manual Control ---
export const manualControlMode = writable<'xbox' | 'keyboard'>('xbox');
export const manualControlActive = writable(false);
export const controlTeam = writable(0);
export const controlRobotId = writable(0);
export const velScaleVx = writable(1.0);
export const velScaleVy = writable(1.0);
export const velScaleW = writable(1.0);
export const visualizeVelocities = writable(false);
export const gamepadStatus = writable('No controller');
export const gamepadConnected = writable(false);

// --- Path Following ---
export const pathDrawMode = writable(false);
export const pathController = writable('pid');
export const pathTeam = writable(0);
export const pathRobotId = writable(0);
export const pathTraceMode = writable(false);
export const pathPoints = writable<{ x: number; y: number }[]>([]);
export const robotTrace = writable<{ x: number; y: number }[]>([]);

// --- Velocity Charts ---
const VEL_CHART_SIZE = 600;
export const velHistory = writable({
	vx: new Array(VEL_CHART_SIZE).fill(0) as number[],
	vy: new Array(VEL_CHART_SIZE).fill(0) as number[],
	omega: new Array(VEL_CHART_SIZE).fill(0) as number[]
});

export function pushVelSample(vx: number, vy: number, omega: number) {
	velHistory.update((h) => ({
		vx: [...h.vx.slice(1), vx],
		vy: [...h.vy.slice(1), vy],
		omega: [...h.omega.slice(1), omega]
	}));
}

// --- Recording ---
export const recFilename = writable('record.csv');
export const recStatus = writable<'idle' | 'recording' | 'saved'>('idle');
export const recStartDisabled = writable(false);
export const recStopDisabled = writable(true);

// --- Script ---
export const scriptPath = writable('');
export const scriptStatus = writable<'idle' | 'loaded' | 'running' | 'paused' | 'error'>('idle');

// --- Lua Draw Commands ---
export interface LuaDrawPoint { type: 'Point'; x: number; y: number }
export interface LuaDrawHighlight { type: 'HighlightRobot'; id: number; team: number }
export interface LuaDrawLine { type: 'Line'; points: [number, number][] }
export type LuaDrawCommand = LuaDrawPoint | LuaDrawHighlight | LuaDrawLine;
export const luaDrawCommands = writable<LuaDrawCommand[]>([]);

// --- Position History (for bottom panel plots) ---
const POS_HISTORY_SIZE = 600;
export const posHistory = writable({
	x: new Array(POS_HISTORY_SIZE).fill(0) as number[],
	y: new Array(POS_HISTORY_SIZE).fill(0) as number[],
	theta: new Array(POS_HISTORY_SIZE).fill(0) as number[]
});

export function pushPosSample(x: number, y: number, theta: number) {
	posHistory.update((h) => ({
		x: [...h.x.slice(1), x],
		y: [...h.y.slice(1), y],
		theta: [...h.theta.slice(1), theta]
	}));
}
