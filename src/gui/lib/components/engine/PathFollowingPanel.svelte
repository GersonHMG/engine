<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { WebviewWindow } from '@tauri-apps/api/window';
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import {
		pathDrawMode,
		pathController,
		pathTeam,
		pathRobotId,
		pathTraceMode,
		pathPoints
	} from '$lib/stores/app.js';

	let drawMode = $state(false);
	let controller = $state('pid');
	let team = $state(0);
	let robotId = $state(0);
	let traceMode = $state(false);

	$effect(() => { pathDrawMode.set(drawMode); });
	$effect(() => { pathController.set(controller); });
	$effect(() => { pathTeam.set(team); });
	$effect(() => { pathRobotId.set(robotId); });
	$effect(() => { pathTraceMode.set(traceMode); });

	function clearPath() {
		pathPoints.set([]);
	}

	async function sendPath() {
		let points: { x: number; y: number }[] = [];
		pathPoints.subscribe((p) => (points = p))();

		if (points.length === 0) {
			alert('No path to send.');
			return;
		}

		const getParam = (key: string, defaultVal: number): number => {
			const val = localStorage.getItem(key);
			return val !== null ? parseFloat(val) : defaultVal;
		};

		const params = {
			lat_kp: getParam('ctrl-lat-kp', 3.0),
			lat_ki: getParam('ctrl-lat-ki', 0.1),
			lat_kd: getParam('ctrl-lat-kd', 0.5),
			speed_kp: getParam('ctrl-speed-kp', 2.0),
			head_kp: getParam('ctrl-head-kp', 4.0),
			target_speed: getParam('ctrl-target-speed', 1.0),
			lookahead: getParam('ctrl-lookahead', 0.25),
			bangbang_a_max: getParam('ctrl-bb-amax', 2.5),
			bangbang_v_max: getParam('ctrl-bb-vmax', 5.0),
			pid_kp: getParam('ctrl-pid-kp', 2.0),
			pid_max_v: getParam('ctrl-pid-maxv', 1.5)
		};

		try {
			await invoke('send_path_test', {
				id: robotId,
				team: team,
				controller: controller,
				params,
				points
			});
			console.log(`Path sent using ${controller} to Team ${team} ID ${robotId}`);
		} catch (e) {
			console.error('Failed to send path:', e);
			alert('Path sent (simulated or error details in console):\n' + e);
		}
	}

	function openSettings() {
		const webview = new WebviewWindow('hyperparameters', {
			url: `hyperparameters?controller=${encodeURIComponent(controller)}`,
			title: `Settings: ${controller}`,
			width: 380,
			height: 520,
			resizable: false
		});
		webview.once('tauri://error', (e: unknown) => {
			console.error('Error creating window. It might already exist.', e);
		});
	}
</script>

<Card>
	<CardHeader>
		<CardTitle>Path Following Test</CardTitle>
	</CardHeader>
	<CardContent class="space-y-3">
		<div class="flex items-center justify-between">
			<Label>Draw Mode</Label>
			<Switch bind:checked={drawMode} />
		</div>
		<div class="flex items-center justify-between">
			<Label>Team</Label>
			<select
				bind:value={team}
				class="h-8 rounded-md border border-input bg-background px-2 text-sm"
			>
				<option value={0}>Blue</option>
				<option value={1}>Yellow</option>
			</select>
		</div>
		<div class="flex items-center justify-between">
			<Label>Robot ID</Label>
			<Input type="number" bind:value={robotId} min={0} max={15} class="w-16 text-right" />
		</div>
		<div class="flex items-center justify-between">
			<Label>Trace Path</Label>
			<Switch bind:checked={traceMode} />
		</div>
		<div class="flex items-center justify-between">
			<Label>Controller</Label>
			<select
				bind:value={controller}
				class="h-8 rounded-md border border-input bg-background px-2 text-sm"
			>
				<option value="pid">PID</option>
				<option value="lookahead">LookAhead PID</option>
				<option value="bangbang">BangBangTrajectories</option>
			</select>
		</div>
		<Button variant="secondary" class="w-full" onclick={openSettings}>
			⚙️ Settings
		</Button>
		<div class="flex gap-2">
			<Button variant="outline" class="flex-1" onclick={clearPath}>Clear Path</Button>
			<Button class="flex-1" onclick={sendPath}>Send</Button>
		</div>
	</CardContent>
</Card>
