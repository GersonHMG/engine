// Manual control service — manages gamepad/keyboard input and sends commands
import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/tauri';
import {
	manualControlActive,
	manualControlMode,
	controlTeam,
	controlRobotId,
	velScaleVx,
	velScaleVy,
	velScaleW,
	gamepadStatus,
	gamepadConnected
} from '$lib/stores/app.js';

let gamepadIndex: number | null = null;
let wasSendingManual = false;

const keys: Record<string, boolean> = {
	w: false,
	a: false,
	s: false,
	d: false,
	q: false,
	e: false
};

export function setupControlListeners() {
	const onGamepadConnected = (e: GamepadEvent) => {
		gamepadIndex = e.gamepad.index;
		gamepadStatus.set('Controller connected');
		gamepadConnected.set(true);
	};

	const onGamepadDisconnected = (e: GamepadEvent) => {
		if (gamepadIndex === e.gamepad.index) {
			gamepadIndex = null;
			gamepadStatus.set('No controller');
			gamepadConnected.set(false);
		}
	};

	const onKeyDown = (e: KeyboardEvent) => {
		const key = e.key.toLowerCase();
		if (key in keys) keys[key] = true;
	};

	const onKeyUp = (e: KeyboardEvent) => {
		const key = e.key.toLowerCase();
		if (key in keys) keys[key] = false;
	};

	window.addEventListener('gamepadconnected', onGamepadConnected);
	window.addEventListener('gamepaddisconnected', onGamepadDisconnected);
	window.addEventListener('keydown', onKeyDown);
	window.addEventListener('keyup', onKeyUp);

	return () => {
		window.removeEventListener('gamepadconnected', onGamepadConnected);
		window.removeEventListener('gamepaddisconnected', onGamepadDisconnected);
		window.removeEventListener('keydown', onKeyDown);
		window.removeEventListener('keyup', onKeyUp);
	};
}

export function pollManualControl(): boolean {
	const isActive = get(manualControlActive);
	if (!isActive) {
		return false;
	}

	const mode = get(manualControlMode);
	let scaleVx = get(velScaleVx);
	let scaleVy = get(velScaleVy);
	let scaleW = get(velScaleW);

	if (isNaN(scaleVx)) scaleVx = 1.0;
	if (isNaN(scaleVy)) scaleVy = 1.0;
	if (isNaN(scaleW)) scaleW = 1.0;

	let vx = 0;
	let vy = 0;
	let omega = 0;

	if (mode === 'xbox') {
		if (gamepadIndex !== null) {
			const gp = navigator.getGamepads()[gamepadIndex];
			if (gp) {
				if (Math.abs(gp.axes[1]) > 0.1) vx = -gp.axes[1] * 2.0;
				if (Math.abs(gp.axes[0]) > 0.1) vy = -gp.axes[0] * 2.0;
				if (Math.abs(gp.axes[2]) > 0.1) omega = -gp.axes[2] * 4.0;
			}
		}
	} else if (mode === 'keyboard') {
		if (keys['w']) vx += 2.0;
		if (keys['s']) vx -= 2.0;
		if (keys['a']) vy += 2.0;
		if (keys['d']) vy -= 2.0;
		if (keys['q']) omega += 4.0;
		if (keys['e']) omega -= 4.0;
	}

	vx *= scaleVx;
	vy *= scaleVy;
	omega *= scaleW;

	if (Math.abs(vx) < 0.05 && Math.abs(vy) < 0.05 && Math.abs(omega) < 0.05) {
		vx = 0;
		vy = 0;
		omega = 0;
		if (!wasSendingManual) return isActive;
		wasSendingManual = false;
	} else {
		wasSendingManual = true;
	}

	const cmdId = get(controlRobotId);
	const cmdTeam = get(controlTeam);

	invoke('send_robot_command', {
		id: cmdId,
		team: cmdTeam,
		vx,
		vy,
		omega
	}).catch(console.error);

	return isActive;
}
