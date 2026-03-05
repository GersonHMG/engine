<script lang="ts">
	import type { Props } from './index.js';
	import { Button as ButtonPrimitive } from 'bits-ui';
	import { buttonVariants } from './index.js';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		variant = 'default',
		size = 'default',
		children,
		ref = $bindable(null),
		...restProps
	}: Props = $props();
</script>

<ButtonPrimitive.Root
	bind:ref
	class={cn(buttonVariants({ variant, size }), className)}
	{...restProps}
>
	{@render children?.()}
</ButtonPrimitive.Root>
