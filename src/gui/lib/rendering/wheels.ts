// Wheel velocity computation and visualization

const WHEEL_ANGLES_DEG = [45, 135, 225, 315];
const WHEEL_ANGLES = WHEEL_ANGLES_DEG.map((d) => (d * Math.PI) / 180);
const L = 0.085; // robot radius in meters

export function computeWheelVelocities(vx: number, vy: number, omega: number): number[] {
	return WHEEL_ANGLES.map((theta) => {
		return -Math.sin(theta) * vx + Math.cos(theta) * vy + L * omega;
	});
}

export function drawWheelViz(
	ctx: CanvasRenderingContext2D,
	W: number,
	H: number,
	currentVx: number,
	currentVy: number,
	currentOmega: number,
	wheelVels: number[]
) {
	const cx = W / 2;
	const cy = H / 2;
	const R = 90;

	ctx.clearRect(0, 0, W, H);

	// Body
	ctx.fillStyle = '#2a2a3e';
	ctx.strokeStyle = '#555';
	ctx.lineWidth = 2;
	ctx.beginPath();
	ctx.arc(cx, cy, R, 0, Math.PI * 2);
	ctx.fill();
	ctx.stroke();

	// Forward marker
	ctx.strokeStyle = '#888';
	ctx.lineWidth = 2;
	ctx.beginPath();
	ctx.moveTo(cx, cy);
	ctx.lineTo(cx + R * 0.95, cy);
	ctx.stroke();

	ctx.fillStyle = '#777';
	ctx.font = '10px monospace';
	ctx.textAlign = 'center';
	ctx.textBaseline = 'middle';
	ctx.fillText('FWD', cx + R + 16, cy);

	// Axis cross
	ctx.strokeStyle = '#333';
	ctx.lineWidth = 1;
	ctx.setLineDash([3, 5]);
	ctx.beginPath();
	ctx.moveTo(cx - R - 15, cy);
	ctx.lineTo(cx + R + 10, cy);
	ctx.moveTo(cx, cy - R - 15);
	ctx.lineTo(cx, cy + R + 10);
	ctx.stroke();
	ctx.setLineDash([]);

	// Wheels
	const wheelPixelDist = R - 5;
	const wheelW = 30;
	const wheelH = 12;
	const maxWv = Math.max(0.1, ...wheelVels.map(Math.abs));

	for (let i = 0; i < 4; i++) {
		const angle = WHEEL_ANGLES[i];
		const wx = cx + Math.cos(angle) * wheelPixelDist;
		const wy = cy - Math.sin(angle) * wheelPixelDist;

		const wv = wheelVels[i];
		const normVel = wv / maxWv;
		const intensity = Math.min(1, Math.abs(normVel));

		let r: number, g: number, b: number;
		if (wv >= 0) {
			r = Math.round(40 + (1 - intensity) * 30);
			g = Math.round(80 + intensity * 175);
			b = Math.round(40 + (1 - intensity) * 30);
		} else {
			r = Math.round(80 + intensity * 175);
			g = Math.round(40 + (1 - intensity) * 30);
			b = Math.round(40 + (1 - intensity) * 30);
		}

		ctx.save();
		ctx.translate(wx, wy);
		ctx.rotate(-angle + Math.PI / 2);

		ctx.fillStyle = `rgb(${r},${g},${b})`;
		ctx.strokeStyle = '#999';
		ctx.lineWidth = 1;
		ctx.fillRect(-wheelW / 2, -wheelH / 2, wheelW, wheelH);
		ctx.strokeRect(-wheelW / 2, -wheelH / 2, wheelW, wheelH);

		// Roller stripes
		ctx.strokeStyle = 'rgba(0,0,0,0.3)';
		ctx.lineWidth = 1;
		for (let s = -wheelW / 2 + 5; s < wheelW / 2; s += 6) {
			ctx.beginPath();
			ctx.moveTo(s, -wheelH / 2);
			ctx.lineTo(s, wheelH / 2);
			ctx.stroke();
		}
		ctx.restore();

		// Velocity arrow
		if (Math.abs(wv) > 0.01) {
			const arrowLen = Math.abs(normVel) * 40 + 10;
			const tangentAngle = angle + Math.PI / 2;
			const dir = wv >= 0 ? 1 : -1;
			const ax = Math.cos(tangentAngle) * dir;
			const ay = -Math.sin(tangentAngle) * dir;

			const startX = wx + ax * (wheelH / 2 + 3);
			const startY = wy + ay * (wheelH / 2 + 3);
			const endX = startX + ax * arrowLen;
			const endY = startY + ay * arrowLen;

			ctx.strokeStyle = wv >= 0 ? '#4f4' : '#f44';
			ctx.lineWidth = 2;
			ctx.beginPath();
			ctx.moveTo(startX, startY);
			ctx.lineTo(endX, endY);
			ctx.stroke();

			const headLen = 7;
			const headAngle = Math.atan2(endY - startY, endX - startX);
			ctx.beginPath();
			ctx.moveTo(endX, endY);
			ctx.lineTo(
				endX - headLen * Math.cos(headAngle - 0.4),
				endY - headLen * Math.sin(headAngle - 0.4)
			);
			ctx.moveTo(endX, endY);
			ctx.lineTo(
				endX - headLen * Math.cos(headAngle + 0.4),
				endY - headLen * Math.sin(headAngle + 0.4)
			);
			ctx.stroke();
		}

		// Numeric label
		const labelDist = R + 35;
		const lx = cx + Math.cos(angle) * labelDist;
		const ly = cy - Math.sin(angle) * labelDist;

		ctx.fillStyle = '#ccc';
		ctx.font = 'bold 11px monospace';
		ctx.textAlign = 'center';
		ctx.textBaseline = 'middle';
		ctx.fillText(`W${i + 1}`, lx, ly - 8);
		ctx.fillStyle = Math.abs(wv) < 0.01 ? '#666' : wv >= 0 ? '#4f4' : '#f44';
		ctx.font = '11px monospace';
		ctx.fillText(wv.toFixed(2), lx, ly + 6);
	}

	// Center label
	ctx.fillStyle = '#666';
	ctx.font = '10px monospace';
	ctx.textAlign = 'center';
	ctx.textBaseline = 'middle';
	ctx.fillText('(top view)', cx, cy + R + 22);

	// Robot velocity arrow
	const bodySpeed = Math.sqrt(currentVx * currentVx + currentVy * currentVy);
	if (bodySpeed > 0.02) {
		const maxArrow = 60;
		const arrowScale = Math.min(1, bodySpeed / 3.0);
		const arrowLength = arrowScale * maxArrow + 8;
		const dirX = currentVx / bodySpeed;
		const dirY = -currentVy / bodySpeed;
		const ex = cx + dirX * arrowLength;
		const ey = cy + dirY * arrowLength;

		ctx.strokeStyle = 'rgba(255,255,0,0.6)';
		ctx.lineWidth = 2.5;
		ctx.beginPath();
		ctx.moveTo(cx, cy);
		ctx.lineTo(ex, ey);
		ctx.stroke();

		const ha = Math.atan2(ey - cy, ex - cx);
		ctx.beginPath();
		ctx.moveTo(ex, ey);
		ctx.lineTo(ex - 8 * Math.cos(ha - 0.4), ey - 8 * Math.sin(ha - 0.4));
		ctx.moveTo(ex, ey);
		ctx.lineTo(ex - 8 * Math.cos(ha + 0.4), ey - 8 * Math.sin(ha + 0.4));
		ctx.stroke();
	}

	// Omega arc
	if (Math.abs(currentOmega) > 0.05) {
		const arcR = R * 0.55;
		const arcSpan = Math.min(Math.PI * 1.5, (Math.abs(currentOmega) / 4) * Math.PI);
		const startA = -Math.PI / 2;
		const endA = currentOmega > 0 ? startA - arcSpan : startA + arcSpan;

		ctx.strokeStyle = 'rgba(100,200,255,0.5)';
		ctx.lineWidth = 2;
		ctx.beginPath();
		ctx.arc(cx, cy, arcR, startA, endA, currentOmega > 0);
		ctx.stroke();

		const tipAngle = endA;
		const tipX = cx + Math.cos(tipAngle) * arcR;
		const tipY = cy + Math.sin(tipAngle) * arcR;
		const tangent =
			currentOmega > 0 ? tipAngle - Math.PI / 2 : tipAngle + Math.PI / 2;
		ctx.beginPath();
		ctx.moveTo(tipX, tipY);
		ctx.lineTo(
			tipX - 6 * Math.cos(tangent - 0.5),
			tipY - 6 * Math.sin(tangent - 0.5)
		);
		ctx.moveTo(tipX, tipY);
		ctx.lineTo(
			tipX - 6 * Math.cos(tangent + 0.5),
			tipY - 6 * Math.sin(tangent + 0.5)
		);
		ctx.stroke();
	}
}
