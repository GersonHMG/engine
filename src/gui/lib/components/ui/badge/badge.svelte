<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { type VariantProps, tv } from 'tailwind-variants';
	import type { HTMLAttributes } from 'svelte/elements';

	const badgeVariants = tv({
		base: 'inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
		variants: {
			variant: {
				default: 'border-transparent bg-primary text-primary-foreground',
				secondary: 'border-transparent bg-secondary text-secondary-foreground',
				destructive: 'border-transparent bg-destructive text-destructive-foreground',
				outline: 'text-foreground'
			}
		},
		defaultVariants: {
			variant: 'default'
		}
	});

	type Variant = VariantProps<typeof badgeVariants>['variant'];

	let {
		class: className,
		variant = 'default',
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & { variant?: Variant; children?: any } = $props();
</script>

<div class={cn(badgeVariants({ variant }), className)} {...restProps}>
	{@render children?.()}
</div>
