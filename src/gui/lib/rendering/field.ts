// Field rendering — pure functions operating on a canvas context

export interface Viewport {
	width: number;
	height: number;
	panX: number;
	panY: number;
	scale: number;
}

const FIELD_LENGTH = 9000;
const FIELD_WIDTH = 6000;

export function drawField(ctx: CanvasRenderingContext2D, vp: Viewport) {
	const { width: w, height: h, panX, panY, scale } = vp;
	const cx = w / 2 + panX;
	const cy = h / 2 + panY;

	ctx.clearRect(0, 0, w, h);

	ctx.strokeStyle = 'white';
	ctx.lineWidth = 2;
	ctx.strokeRect(
		cx - (FIELD_LENGTH / 2) * scale,
		cy - (FIELD_WIDTH / 2) * scale,
		FIELD_LENGTH * scale,
		FIELD_WIDTH * scale
	);

	ctx.beginPath();
	ctx.arc(cx, cy, 500 * scale, 0, Math.PI * 2);
	ctx.stroke();

	ctx.beginPath();
	ctx.moveTo(cx, cy - (FIELD_WIDTH / 2) * scale);
	ctx.lineTo(cx, cy + (FIELD_WIDTH / 2) * scale);
	ctx.stroke();
}

export function drawRobot(
	ctx: CanvasRenderingContext2D,
	vp: Viewport,
	x: number,
	y: number,
	theta: number,
	teamColor: string,
	id: number,
	actualVx = 0,
	actualVy = 0,
	cmdVx = 0,
	cmdVy = 0
) {
	const { width: w, height: h, panX, panY, scale } = vp;
	const cx = w / 2 + panX;
	const cy = h / 2 + panY;

	const screenX = cx + x * 1000 * scale;
	const screenY = cy - y * 1000 * scale;

	ctx.save();
	ctx.translate(screenX, screenY);
	ctx.rotate(-theta);

	ctx.fillStyle = teamColor;
	ctx.beginPath();
	ctx.arc(0, 0, 90 * scale, 0, Math.PI * 2);
	ctx.fill();

	// Omni wheels
	ctx.fillStyle = 'black';
	const wheelW = 30 * scale;
	const wheelH = 10 * scale;
	const wheelDist = 85 * scale;
	const angles = [Math.PI / 4, (3 * Math.PI) / 4, (5 * Math.PI) / 4, (7 * Math.PI) / 4];

	angles.forEach((a) => {
		ctx.save();
		ctx.rotate(a);
		ctx.translate(wheelDist, 0);
		ctx.fillRect(-wheelH / 2, -wheelW / 2, wheelH, wheelW);
		ctx.restore();
	});

	ctx.strokeStyle = 'black';
	ctx.lineWidth = 2;
	ctx.beginPath();
	ctx.moveTo(0, 0);
	ctx.lineTo(90 * scale, 0);
	ctx.stroke();

	ctx.fillStyle = 'white';
	ctx.font = `${Math.max(10, 12 * (scale / 0.08))}px Arial`;
	ctx.textAlign = 'center';
	ctx.textBaseline = 'middle';
	ctx.fillText(String(id), 0, 0);

	ctx.restore();

	// Velocity vectors
	const VEL_SCALE = 1000 * scale;

	// Actual velocity (red)
	if (Math.abs(actualVx) > 0.05 || Math.abs(actualVy) > 0.05) {
		ctx.beginPath();
		ctx.strokeStyle = 'rgba(255, 0, 0, 0.5)';
		ctx.fillStyle = 'rgba(255, 0, 0, 0.5)';
		ctx.lineWidth = 3;
		ctx.moveTo(screenX, screenY);
		const endX = screenX + actualVx * VEL_SCALE;
		const endY = screenY - actualVy * VEL_SCALE;
		ctx.lineTo(endX, endY);
		ctx.stroke();
		ctx.fillRect(endX - 3, endY - 3, 6, 6);
	}

	// Commanded velocity (green, local→global)
	if (Math.abs(cmdVx) > 0.05 || Math.abs(cmdVy) > 0.05) {
		ctx.beginPath();
		ctx.strokeStyle = 'rgba(0, 255, 0, 0.5)';
		ctx.fillStyle = 'rgba(0, 255, 0, 0.5)';
		ctx.lineWidth = 3;
		ctx.moveTo(screenX, screenY);
		const gVx = cmdVx * Math.cos(theta) - cmdVy * Math.sin(theta);
		const gVy = cmdVx * Math.sin(theta) + cmdVy * Math.cos(theta);
		const endX = screenX + gVx * VEL_SCALE;
		const endY = screenY - gVy * VEL_SCALE;
		ctx.lineTo(endX, endY);
		ctx.stroke();
		ctx.fillRect(endX - 3, endY - 3, 6, 6);
	}
}

export function drawBall(ctx: CanvasRenderingContext2D, vp: Viewport, x: number, y: number) {
	const { width: w, height: h, panX, panY, scale } = vp;
	const cx = w / 2 + panX;
	const cy = h / 2 + panY;

	const screenX = cx + x * 1000 * scale;
	const screenY = cy - y * 1000 * scale;

	ctx.fillStyle = 'orange';
	ctx.beginPath();
	ctx.arc(screenX, screenY, 25 * scale, 0, Math.PI * 2);
	ctx.fill();
	ctx.strokeStyle = 'black';
	ctx.lineWidth = 0.5;
	ctx.stroke();
}

export function drawPath(
	ctx: CanvasRenderingContext2D,
	vp: Viewport,
	points: { x: number; y: number }[]
) {
	if (points.length === 0) return;
	const { width: w, height: h, panX, panY, scale } = vp;
	const cx = w / 2 + panX;
	const cy = h / 2 + panY;

	ctx.strokeStyle = 'magenta';
	ctx.lineWidth = 2;
	ctx.beginPath();
	for (let i = 0; i < points.length; i++) {
		const screenX = cx + points[i].x * 1000 * scale;
		const screenY = cy - points[i].y * 1000 * scale;
		if (i === 0) ctx.moveTo(screenX, screenY);
		else ctx.lineTo(screenX, screenY);
	}
	ctx.stroke();

	ctx.fillStyle = 'magenta';
	for (let i = 0; i < points.length; i++) {
		const screenX = cx + points[i].x * 1000 * scale;
		const screenY = cy - points[i].y * 1000 * scale;
		ctx.beginPath();
		ctx.arc(screenX, screenY, 4, 0, Math.PI * 2);
		ctx.fill();
	}
}

export function drawRobotTrace(
	ctx: CanvasRenderingContext2D,
	vp: Viewport,
	points: { x: number; y: number }[]
) {
	if (points.length === 0) return;
	const { width: w, height: h, panX, panY, scale } = vp;
	const cx = w / 2 + panX;
	const cy = h / 2 + panY;

	ctx.strokeStyle = 'cyan';
	ctx.lineWidth = 2;
	ctx.beginPath();
	for (let i = 0; i < points.length; i++) {
		const screenX = cx + points[i].x * 1000 * scale;
		const screenY = cy - points[i].y * 1000 * scale;
		if (i === 0) ctx.moveTo(screenX, screenY);
		else ctx.lineTo(screenX, screenY);
	}
	ctx.stroke();
}

export function drawDisconnectedOverlay(ctx: CanvasRenderingContext2D, w: number, h: number) {
	ctx.fillStyle = 'rgba(128, 128, 128, 0.5)';
	ctx.fillRect(0, 0, w, h);
	ctx.fillStyle = 'white';
	ctx.font = '30px Arial';
	ctx.textAlign = 'center';
	ctx.textBaseline = 'middle';
	ctx.fillText('No Vision Connected', w / 2, h / 2);
}

export function drawLuaCommands(
	ctx: CanvasRenderingContext2D,
	vp: Viewport,
	commands: { type: string; x?: number; y?: number; id?: number; team?: number; points?: [number, number][] }[],
	blue: { id: number; x: number; y: number }[],
	yellow: { id: number; x: number; y: number }[]
) {
	const { width: w, height: h, panX, panY, scale } = vp;
	const cx = w / 2 + panX;
	const cy = h / 2 + panY;

	for (const cmd of commands) {
		switch (cmd.type) {
			case 'Point': {
				const sx = cx + cmd.x! * 1000 * scale;
				const sy = cy - cmd.y! * 1000 * scale;
				ctx.fillStyle = 'lime';
				ctx.beginPath();
				ctx.arc(sx, sy, Math.max(4, 40 * scale), 0, Math.PI * 2);
				ctx.fill();
				break;
			}
			case 'HighlightRobot': {
				const list = cmd.team === 0 ? blue : yellow;
				const robot = list.find((r) => r.id === cmd.id);
				if (robot) {
					const sx = cx + robot.x * 1000 * scale;
					const sy = cy - robot.y * 1000 * scale;
					ctx.strokeStyle = 'lime';
					ctx.lineWidth = 3;
					ctx.beginPath();
					ctx.arc(sx, sy, 120 * scale, 0, Math.PI * 2);
					ctx.stroke();
				}
				break;
			}
			case 'Line': {
				const pts = cmd.points!;
				if (pts.length < 2) break;
				ctx.strokeStyle = 'lime';
				ctx.lineWidth = 2;
				ctx.beginPath();
				for (let i = 0; i < pts.length; i++) {
					const sx = cx + pts[i][0] * 1000 * scale;
					const sy = cy - pts[i][1] * 1000 * scale;
					if (i === 0) ctx.moveTo(sx, sy);
					else ctx.lineTo(sx, sy);
				}
				ctx.stroke();
				ctx.fillStyle = 'lime';
				for (const pt of pts) {
					const sx = cx + pt[0] * 1000 * scale;
					const sy = cy - pt[1] * 1000 * scale;
					ctx.beginPath();
					ctx.arc(sx, sy, Math.max(3, 30 * scale), 0, Math.PI * 2);
					ctx.fill();
				}
				break;
			}
		}
	}
}

export function screenToField(
	vp: Viewport,
	clientX: number,
	clientY: number,
	canvasRect: DOMRect
): { x: number; y: number } {
	const cx = vp.width / 2 + vp.panX;
	const cy = vp.height / 2 + vp.panY;
	const mouseX = clientX - canvasRect.left;
	const mouseY = clientY - canvasRect.top;
	return {
		x: (mouseX - cx) / vp.scale / 1000.0,
		y: -(mouseY - cy) / vp.scale / 1000.0
	};
}

export function screenToFieldMm(
	vp: Viewport,
	clientX: number,
	clientY: number,
	canvasRect: DOMRect
): { x: number; y: number } {
	const cx = vp.width / 2 + vp.panX;
	const cy = vp.height / 2 + vp.panY;
	const mouseX = clientX - canvasRect.left;
	const mouseY = clientY - canvasRect.top;
	return {
		x: Math.round((mouseX - cx) / vp.scale),
		y: Math.round(-(mouseY - cy) / vp.scale)
	};
}
