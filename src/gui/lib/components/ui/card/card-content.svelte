<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & { children?: any } = $props();
</script>

<div class={cn('p-3 pt-0', className)} {...restProps}>
	{@render children?.()}
</div>
