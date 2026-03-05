// Chart rendering — PPS graph and velocity command charts

export function drawPpsGraph(
	ctx: CanvasRenderingContext2D,
	w: number,
	h: number,
	history: number[]
) {
	ctx.clearRect(0, 0, w, h);

	ctx.strokeStyle = '#0f0';
	ctx.lineWidth = 1;
	ctx.beginPath();

	const maxVal = Math.max(...history, 10);
	const currentMax = Math.max(100, maxVal);

	for (let i = 0; i < history.length; i++) {
		const x = (i / (history.length - 1)) * w;
		const y = h - (history[i] / currentMax) * h;
		if (i === 0) ctx.moveTo(x, y);
		else ctx.lineTo(x, y);
	}
	ctx.stroke();
}

export function drawVelPlot(
	ctx: CanvasRenderingContext2D,
	w: number,
	h: number,
	data: number[],
	color: string
): number {
	ctx.clearRect(0, 0, w, h);

	let maxAbs = 0.5;
	for (let i = 0; i < data.length; i++) {
		maxAbs = Math.max(maxAbs, Math.abs(data[i]));
	}
	maxAbs = Math.ceil(maxAbs * 2) / 2;

	// Background
	ctx.fillStyle = '#1a1a2e';
	ctx.fillRect(0, 0, w, h);

	// Zero line
	const zeroY = h / 2;
	ctx.strokeStyle = '#444';
	ctx.lineWidth = 1;
	ctx.setLineDash([3, 3]);
	ctx.beginPath();
	ctx.moveTo(0, zeroY);
	ctx.lineTo(w, zeroY);
	ctx.stroke();
	ctx.setLineDash([]);

	// Y-axis labels
	ctx.fillStyle = '#555';
	ctx.font = '8px monospace';
	ctx.textAlign = 'left';
	ctx.fillText(`+${maxAbs.toFixed(1)}`, 1, 8);
	ctx.fillText(`-${maxAbs.toFixed(1)}`, 1, h - 2);

	// Data line
	ctx.strokeStyle = color;
	ctx.lineWidth = 1.5;
	ctx.beginPath();
	for (let i = 0; i < data.length; i++) {
		const x = (i / (data.length - 1)) * w;
		const y = zeroY - (data[i] / maxAbs) * (h / 2);
		if (i === 0) ctx.moveTo(x, y);
		else ctx.lineTo(x, y);
	}
	ctx.stroke();

	// Fill area
	ctx.lineTo(w, zeroY);
	ctx.lineTo(0, zeroY);
	ctx.closePath();
	ctx.fillStyle = color.replace(')', ', 0.08)').replace('rgb', 'rgba');
	ctx.fill();

	// Return last value
	return data[data.length - 1];
}
