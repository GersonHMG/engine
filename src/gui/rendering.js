// rendering.js
const canvas = document.getElementById('fieldCanvas');
const ctx = canvas ? canvas.getContext('2d') : null;
const mouseCoords = document.getElementById('mouse-coords');
import { addPathPoint } from './control.js';

const FIELD_LENGTH = 9000;
const FIELD_WIDTH = 6000;

// --- Zoom Logic ---
export let scale = 0.08; // Initial pixels per mm
const MIN_SCALE = 0.01;
const MAX_SCALE = 0.5;

// --- Pan Logic ---
export let panX = 0; // Offset in pixels
export let panY = 0;
let isDragging = false;
let didDrag = false;
let lastMouseX = 0;
let lastMouseY = 0;

export function initRendering() {
    if (!canvas) return;

    const btnZoomIn = document.getElementById('btn-zoom-in');
    const btnZoomOut = document.getElementById('btn-zoom-out');

    if (btnZoomIn) {
        btnZoomIn.addEventListener('click', () => {
            scale = Math.min(MAX_SCALE, scale * 1.2);
            drawField();
            updateMouseCoords(lastMouseX, lastMouseY);
        });
    }

    if (btnZoomOut) {
        btnZoomOut.addEventListener('click', () => {
            scale = Math.max(MIN_SCALE, scale / 1.2);
            drawField();
            updateMouseCoords(lastMouseX, lastMouseY);
        });
    }

    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        const zoomFactor = 1.1;
        if (e.deltaY < 0) {
            scale = Math.min(MAX_SCALE, scale * zoomFactor);
        } else {
            scale = Math.max(MIN_SCALE, scale / zoomFactor);
        }
        drawField();
        updateMouseCoords(e.clientX, e.clientY);
    });

    canvas.addEventListener('mousedown', (e) => {
        isDragging = true;
        didDrag = false;
        lastMouseX = e.clientX;
        lastMouseY = e.clientY;
        canvas.style.cursor = 'grabbing';
    });

    window.addEventListener('mousemove', (e) => {
        updateMouseCoords(e.clientX, e.clientY);

        if (!isDragging) return;

        const dx = e.clientX - lastMouseX;
        const dy = e.clientY - lastMouseY;

        if (Math.abs(dx) > 2 || Math.abs(dy) > 2) {
            didDrag = true;
        }

        panX += dx;
        panY += dy;
        lastMouseX = e.clientX;
        lastMouseY = e.clientY;
        drawField();
    });

    window.addEventListener('mouseup', (e) => {
        if (isDragging && !didDrag && e.target === canvas) {
            const w = canvas.width;
            const h = canvas.height;
            const cx = (w / 2) + panX;
            const cy = (h / 2) + panY;

            const rect = canvas.getBoundingClientRect();
            const mouseX = e.clientX - rect.left;
            const mouseY = e.clientY - rect.top;

            const fieldX = (mouseX - cx) / scale / 1000.0;
            const fieldY = -(mouseY - cy) / scale / 1000.0;

            addPathPoint(fieldX, fieldY);
        }

        isDragging = false;
        canvas.style.cursor = 'grab';
    });

    window.addEventListener('resize', resize);
    resize();
    canvas.style.cursor = 'grab';
}

function updateMouseCoords(clientX, clientY) {
    if (!canvas) return;
    const w = canvas.width;
    const h = canvas.height;
    const cx = (w / 2) + panX;
    const cy = (h / 2) + panY;

    const rect = canvas.getBoundingClientRect();
    const mouseX = clientX - rect.left;
    const mouseY = clientY - rect.top;

    const fieldX = Math.round((mouseX - cx) / scale);
    const fieldY = Math.round(-(mouseY - cy) / scale);

    if (mouseCoords) {
        mouseCoords.textContent = `${fieldX}, ${fieldY}`;
    }
}

export function drawField() {
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;
    const cx = (w / 2) + panX;
    const cy = (h / 2) + panY;

    ctx.clearRect(0, 0, w, h);

    ctx.strokeStyle = 'white';
    ctx.lineWidth = 2;
    ctx.strokeRect(cx - (FIELD_LENGTH / 2 * scale), cy - (FIELD_WIDTH / 2 * scale), FIELD_LENGTH * scale, FIELD_WIDTH * scale);

    ctx.beginPath();
    ctx.arc(cx, cy, 500 * scale, 0, Math.PI * 2);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(cx, cy - (FIELD_WIDTH / 2 * scale));
    ctx.lineTo(cx, cy + (FIELD_WIDTH / 2 * scale));
    ctx.stroke();
}

export function drawRobot(x, y, theta, teamColor, id, actualVx = 0, actualVy = 0, cmdVx = 0, cmdVy = 0) {
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;
    const cx = (w / 2) + panX;
    const cy = (h / 2) + panY;

    const screenX = cx + x * 1000 * scale;
    const screenY = cy - y * 1000 * scale;

    ctx.save();
    ctx.translate(screenX, screenY);
    ctx.rotate(-theta);

    ctx.fillStyle = teamColor;
    ctx.beginPath();
    ctx.arc(0, 0, 90 * scale, 0, Math.PI * 2);
    ctx.fill();

    // Omni Wheels
    ctx.fillStyle = 'black';
    const wheelW = 30 * scale;
    const wheelH = 10 * scale;
    const wheelDist = 85 * scale;
    const angles = [Math.PI / 4, 3 * Math.PI / 4, 5 * Math.PI / 4, 7 * Math.PI / 4];

    angles.forEach(a => {
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
    ctx.fillText(id, 0, 0);

    ctx.restore();

    // Visual Vectors
    const VEL_SCALE = 1000 * scale; // 1 m/s = 1000 mm

    // Actual velocity (red) (Global coordinates)
    if (Math.abs(actualVx) > 0.05 || Math.abs(actualVy) > 0.05) {
        ctx.beginPath();
        ctx.strokeStyle = 'rgba(255, 0, 0, 0.5)'; // red 50%
        ctx.fillStyle = 'rgba(255, 0, 0, 0.5)';
        ctx.lineWidth = 3;
        ctx.moveTo(screenX, screenY);
        let endX = screenX + actualVx * VEL_SCALE;
        let endY = screenY - actualVy * VEL_SCALE;
        ctx.lineTo(endX, endY);
        ctx.stroke();

        ctx.fillRect(endX - 3, endY - 3, 6, 6);
    }

    // Commanded velocity (green) (Local coordinates)
    if (Math.abs(cmdVx) > 0.05 || Math.abs(cmdVy) > 0.05) {
        ctx.beginPath();
        ctx.strokeStyle = 'rgba(0, 255, 0, 0.5)'; // green 50%
        ctx.fillStyle = 'rgba(0, 255, 0, 0.5)';
        ctx.lineWidth = 3;
        ctx.moveTo(screenX, screenY);

        // Transform local commanded to global space for drawing
        const gVx = cmdVx * Math.cos(theta) - cmdVy * Math.sin(theta);
        const gVy = cmdVx * Math.sin(theta) + cmdVy * Math.cos(theta);

        let endX = screenX + gVx * VEL_SCALE;
        let endY = screenY - gVy * VEL_SCALE;
        ctx.lineTo(endX, endY);
        ctx.stroke();

        ctx.fillRect(endX - 3, endY - 3, 6, 6);
    }
}

export function drawPath(points) {
    if (!ctx || points.length === 0) return;
    const w = canvas.width;
    const h = canvas.height;
    const cx = (w / 2) + panX;
    const cy = (h / 2) + panY;

    ctx.strokeStyle = 'magenta';
    ctx.lineWidth = 2;
    ctx.beginPath();

    for (let i = 0; i < points.length; i++) {
        const screenX = cx + points[i].x * 1000 * scale;
        const screenY = cy - points[i].y * 1000 * scale;
        if (i === 0) {
            ctx.moveTo(screenX, screenY);
        } else {
            ctx.lineTo(screenX, screenY);
        }
    }
    ctx.stroke();

    // Draw waypoints
    ctx.fillStyle = 'magenta';
    for (let i = 0; i < points.length; i++) {
        const screenX = cx + points[i].x * 1000 * scale;
        const screenY = cy - points[i].y * 1000 * scale;
        ctx.beginPath();
        ctx.arc(screenX, screenY, 4, 0, Math.PI * 2);
        ctx.fill();
    }
}

export function drawBall(x, y) {
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;
    const cx = (w / 2) + panX;
    const cy = (h / 2) + panY;

    const screenX = cx + x * 1000 * scale;
    const screenY = cy - y * 1000 * scale;

    ctx.fillStyle = 'orange';
    ctx.beginPath();
    ctx.arc(screenX, screenY, 25 * scale, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = 'black';
    ctx.lineWidth = 1;
    ctx.stroke();
}

export function resize() {
    if (!canvas) return;
    const main = document.getElementById('main');
    if (main) {
        canvas.width = main.clientWidth;
        canvas.height = main.clientHeight;
    } else {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
    }
    drawField();
}

export const ppsHistory = new Array(50).fill(0);
export let currentPPS = 0;
const ppsCanvas = document.getElementById('pps-graph');
const ppsCtx = ppsCanvas ? ppsCanvas.getContext('2d') : null;

export function setCurrentPPS(val) {
    currentPPS = val;
}

export function drawPPSGraph() {
    if (!ppsCtx) return;
    const w = ppsCanvas.width;
    const h = ppsCanvas.height;
    ppsCtx.clearRect(0, 0, w, h);

    ppsCtx.strokeStyle = '#0f0';
    ppsCtx.lineWidth = 1;
    ppsCtx.beginPath();

    const maxVal = Math.max(...ppsHistory, 10);
    const currentMax = Math.max(100, maxVal);

    for (let i = 0; i < ppsHistory.length; i++) {
        const x = (i / (ppsHistory.length - 1)) * w;
        const y = h - (ppsHistory[i] / currentMax) * h;
        if (i === 0) ppsCtx.moveTo(x, y);
        else ppsCtx.lineTo(x, y);
    }
    ppsCtx.stroke();
}
