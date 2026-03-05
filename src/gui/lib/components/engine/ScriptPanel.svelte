<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { open } from '@tauri-apps/api/dialog';
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { scriptPath, scriptStatus } from '$lib/stores/app.js';

	async function selectScript() {
		const selected = await open({
			filters: [{ name: 'Lua Scripts', extensions: ['lua'] }],
			multiple: false
		});
		if (selected && typeof selected === 'string') {
			scriptPath.set(selected);
			try {
				await invoke('load_script', { path: selected });
				scriptStatus.set('loaded');
			} catch (e) {
				console.error('Failed to load script:', e);
				scriptStatus.set('error');
			}
		}
	}

	async function playScript() {
		try {
			await invoke('resume_script');
			scriptStatus.set('running');
		} catch (e) {
			console.error('Failed to resume script:', e);
		}
	}

	async function pauseScript() {
		try {
			await invoke('pause_script');
			scriptStatus.set('paused');
		} catch (e) {
			console.error('Failed to pause script:', e);
		}
	}

	async function reloadScript() {
		try {
			await invoke('reload_script');
			scriptStatus.set('loaded');
		} catch (e) {
			console.error('Failed to reload script:', e);
		}
	}

	function filename(path: string): string {
		if (!path) return 'None';
		const parts = path.replace(/\\/g, '/').split('/');
		return parts[parts.length - 1];
	}
</script>

<Card>
	<CardHeader>
		<CardTitle>Lua Script</CardTitle>
	</CardHeader>
	<CardContent class="space-y-3">
		<div class="flex items-center justify-between">
			<Label>Script</Label>
			<span class="max-w-[140px] truncate text-xs text-muted-foreground" title={$scriptPath}>
				{filename($scriptPath)}
			</span>
		</div>
		<div class="flex items-center justify-between">
			<Label>Status</Label>
			<Badge
				variant={$scriptStatus === 'running' ? 'default' : $scriptStatus === 'error' ? 'destructive' : 'secondary'}
			>
				{$scriptStatus}
			</Badge>
		</div>
		<div class="grid grid-cols-2 gap-2">
			<Button class="w-full" size="sm" onclick={selectScript}>
				Select Script
			</Button>
			<Button class="w-full" size="sm" onclick={playScript} disabled={!$scriptPath}>
				Play
			</Button>
			<Button class="w-full" size="sm" variant="secondary" onclick={pauseScript} disabled={!$scriptPath}>
				Pause
			</Button>
			<Button class="w-full" size="sm" variant="secondary" onclick={reloadScript} disabled={!$scriptPath}>
				Reload
			</Button>
		</div>
	</CardContent>
</Card>
