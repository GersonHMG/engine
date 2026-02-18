const { invoke } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;

const canvas = document.getElementById('fieldCanvas');
const ctx = canvas.getContext('2d');
const mouseCoords = document.getElementById('mouse-coords');
const visionStatus = document.getElementById('vision-status');

let robots = {
    blue: [],
    yellow: []
};
let ball = { x: 0, y: 0 };
let lastVisionUpdate = 0;

// Field dimensions (mm)
const FIELD_LENGTH = 9000;
const FIELD_WIDTH = 6000;
// --- Zoom Logic ---
let scale = 0.08; // Initial pixels per mm
const MIN_SCALE = 0.01;
const MAX_SCALE = 0.5;

// --- Pan Logic ---
let panX = 0; // Offset in pixels
let panY = 0;
let isDragging = false;
let lastMouseX = 0;
let lastMouseY = 0;

document.getElementById('btn-zoom-in').addEventListener('click', () => {
    scale = Math.min(MAX_SCALE, scale * 1.2);
    drawField();
    updateMouseCoords(lastMouseX, lastMouseY);
});

document.getElementById('btn-zoom-out').addEventListener('click', () => {
    scale = Math.max(MIN_SCALE, scale / 1.2);
    drawField();
    updateMouseCoords(lastMouseX, lastMouseY);
});

// Mouse Wheel Zoom
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

// Drag and Drop (Pan)
canvas.addEventListener('mousedown', (e) => {
    isDragging = true;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
    canvas.style.cursor = 'grabbing';
});

window.addEventListener('mousemove', (e) => {
    updateMouseCoords(e.clientX, e.clientY);

    if (!isDragging) return;
    const dx = e.clientX - lastMouseX;
    const dy = e.clientY - lastMouseY;
    panX += dx;
    panY += dy;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
    drawField();
});

window.addEventListener('mouseup', () => {
    isDragging = false;
    canvas.style.cursor = 'grab';
});

function updateMouseCoords(clientX, clientY) {
    const w = canvas.width;
    const h = canvas.height;
    const cx = (w / 2) + panX;
    const cy = (h / 2) + panY;

    // Calculate field coordinates from screen coordinates
    // screenX = cx + fieldX * scale => fieldX = (screenX - cx) / scale
    // screenY = cy - fieldY * scale => fieldY = -(screenY - cy) / scale

    // We need coordinates relative to the canvas element, not window
    const rect = canvas.getBoundingClientRect();
    const mouseX = clientX - rect.left;
    const mouseY = clientY - rect.top;

    const fieldX = Math.round((mouseX - cx) / scale);
    const fieldY = Math.round(-(mouseY - cy) / scale);

    if (mouseCoords) {
        mouseCoords.textContent = `${fieldX}, ${fieldY}`;
    }
}

// --- Field Drawing ---
function drawField() {
    const w = canvas.width;
    const h = canvas.height;
    // Apply pan offset to center
    const cx = (w / 2) + panX;
    const cy = (h / 2) + panY;

    ctx.clearRect(0, 0, w, h);

    // Draw field boundary
    ctx.strokeStyle = 'white';
    ctx.lineWidth = 2;
    ctx.strokeRect(cx - (FIELD_LENGTH / 2 * scale), cy - (FIELD_WIDTH / 2 * scale), FIELD_LENGTH * scale, FIELD_WIDTH * scale);

    // Center circle
    ctx.beginPath();
    ctx.arc(cx, cy, 500 * scale, 0, Math.PI * 2);
    ctx.stroke();

    // Center line
    ctx.beginPath();
    ctx.moveTo(cx, cy - (FIELD_WIDTH / 2 * scale));
    ctx.lineTo(cx, cy + (FIELD_WIDTH / 2 * scale));
    ctx.stroke();
}

function drawRobot(x, y, theta, teamColor, id) {
    const w = canvas.width;
    const h = canvas.height;
    // Apply pan offset
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

    // Direction
    ctx.strokeStyle = 'black';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(90 * scale, 0);
    ctx.stroke();

    // ID
    ctx.fillStyle = 'white';
    ctx.font = `${Math.max(10, 12 * (scale / 0.08))}px Arial`; // Scale font slightly
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(id, 0, 0);

    ctx.restore();
}

function drawBall(x, y) {
    const w = canvas.width;
    const h = canvas.height;
    // Apply pan offset
    const cx = (w / 2) + panX;
    const cy = (h / 2) + panY;

    const screenX = cx + x * 1000 * scale;
    const screenY = cy - y * 1000 * scale;

    ctx.fillStyle = 'orange';
    ctx.beginPath();
    ctx.arc(screenX, screenY, 25 * scale, 0, Math.PI * 2);
    ctx.fill();
}

function resize() {
    // Resize based on #main container, not window
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
window.addEventListener('resize', resize);
// Initial resize
resize();
// Set initial cursor
canvas.style.cursor = 'grab';

// Listen for updates
// Listen for updates
const ppsHistory = new Array(50).fill(0);
const ppsCanvas = document.getElementById('pps-graph');
const ppsCtx = ppsCanvas ? ppsCanvas.getContext('2d') : null;

listen('vision-update', (event) => {
    const payload = event.payload;
    if (payload.robots_blue) robots.blue = payload.robots_blue;
    if (payload.robots_yellow) robots.yellow = payload.robots_yellow;
    if (payload.ball) ball = payload.ball;

    // Update PPS
    if (payload.pps !== undefined) {
        ppsHistory.shift();
        ppsHistory.push(payload.pps);
        drawPPSGraph();
    }

    lastVisionUpdate = Date.now();
    if (visionStatus) {
        visionStatus.textContent = `Connected (${payload.pps} PPS)`;
        visionStatus.style.color = "#0f0";
    }
});

function drawPPSGraph() {
    if (!ppsCtx) return;
    const w = ppsCanvas.width;
    const h = ppsCanvas.height;
    ppsCtx.clearRect(0, 0, w, h);

    ppsCtx.strokeStyle = '#0f0';
    ppsCtx.lineWidth = 1;
    ppsCtx.beginPath();

    const maxPPS = 100; // Expected max, or dynamic?
    // Let's use dynamic max for scaling
    // const currentMax = Math.max(...ppsHistory, 60); 
    const currentMax = 100;

    for (let i = 0; i < ppsHistory.length; i++) {
        const x = (i / (ppsHistory.length - 1)) * w;
        const y = h - (ppsHistory[i] / currentMax) * h;
        if (i === 0) ppsCtx.moveTo(x, y);
        else ppsCtx.lineTo(x, y);
    }
    ppsCtx.stroke();
}

// ... (PPS drawing logic)

// --- Configuration Logic ---
// --- Configuration Logic ---
const visionIp = document.getElementById('vision-ip');
const visionPort = document.getElementById('vision-port');
// Removed visionLocalIp
const btnReconnect = document.getElementById('btn-reconnect');

btnReconnect.addEventListener('click', async () => {
    try {
        await invoke('update_vision_connection', {
            ip: visionIp.value,
            port: parseInt(visionPort.value)
        });
        console.log("Vision connection updated");
        if (visionStatus) {
            visionStatus.textContent = "Connecting...";
            visionStatus.style.color = "#888";
        }
    } catch (e) {
        console.error("Failed to update vision:", e);
        alert("Failed to reconnect: " + e);
    }
});

// Radio Config
const radioPortName = document.getElementById('radio-port-name');
const radioBaudRate = document.getElementById('radio-baud-rate');
const useRadio = document.getElementById('use-radio');
const btnUpdateRadio = document.getElementById('btn-update-radio');

btnUpdateRadio.addEventListener('click', async () => {
    try {
        await invoke('update_radio_config', {
            useRadio: useRadio.checked,
            portName: radioPortName.value,
            baudRate: parseInt(radioBaudRate.value)
        });
        console.log("Radio config updated");
        alert("Radio configuration updated!");
    } catch (e) {
        console.error("Failed to update radio:", e);
        alert("Failed to update radio: " + e);
    }
});

const kfEnabled = document.getElementById('kf-enabled');
// ... (rest of config logic)
const kfPNoiseP = document.getElementById('kf-pnoise-p');
const kfPNoiseV = document.getElementById('kf-pnoise-v');
const kfMNoise = document.getElementById('kf-mnoise');
const btnUpdateKF = document.getElementById('btn-update-kf');

btnUpdateKF.addEventListener('click', async () => {
    try {
        await invoke('update_tracker_config', {
            enabled: kfEnabled.checked,
            processNoiseP: parseFloat(kfPNoiseP.value),
            processNoiseV: parseFloat(kfPNoiseV.value),
            measurementNoise: parseFloat(kfMNoise.value)
        });
        console.log("KF config updated");
    } catch (e) {
        console.error("Failed to update KF:", e);
    }
});

// --- Recording Logic ---
const recFilename = document.getElementById('rec-filename');
const btnRecordStart = document.getElementById('btn-record-start');
const btnRecordStop = document.getElementById('btn-record-stop');
const recStatus = document.getElementById('rec-status');

btnRecordStart.addEventListener('click', async () => {
    try {
        await invoke('start_recording', { filename: recFilename.value });
        recStatus.textContent = "Recording...";
        recStatus.style.color = "#0f0";
        btnRecordStart.disabled = true;
        btnRecordStop.disabled = false;
    } catch (e) {
        console.error(e);
        alert(e);
    }
});

btnRecordStop.addEventListener('click', async () => {
    try {
        await invoke('stop_recording');
        recStatus.textContent = "Saved";
        recStatus.style.color = "#888";
        btnRecordStart.disabled = false;
        btnRecordStop.disabled = true;
    } catch (e) {
        console.error(e);
    }
});

// --- Xbox Control Logic ---
const xboxActive = document.getElementById('xbox-active');
const xboxTeam = document.getElementById('xbox-team');
const xboxId = document.getElementById('xbox-id');
const xboxStatus = document.getElementById('xbox-status');

let gamepadIndex = null;

window.addEventListener("gamepadconnected", (e) => {
    gamepadIndex = e.gamepad.index;
    xboxStatus.textContent = "Controller connected";
    xboxStatus.style.color = "#0f0";
});

window.addEventListener("gamepaddisconnected", (e) => {
    if (gamepadIndex === e.gamepad.index) {
        gamepadIndex = null;
        xboxStatus.textContent = "No controller";
        xboxStatus.style.color = "#888";
    }
});

// Poll gamepad in loop
function pollGamepad() {
    if (!xboxActive.checked || gamepadIndex === null) return;

    const gp = navigator.getGamepads()[gamepadIndex];
    if (!gp) return;

    // Simple tank drive or arcade drive
    // Axis 1 (Left Stick Y) -> Vx
    // Axis 0 (Left Stick X) -> Vy
    // Axis 2 (Right Stick X) -> Omega

    const vx = -gp.axes[1] * 2.0; // Max 2m/s
    const vy = -gp.axes[0] * 2.0;
    const omega = -gp.axes[2] * 4.0; // Rad/s

    // Deadzone
    if (Math.abs(vx) < 0.1 && Math.abs(vy) < 0.1 && Math.abs(omega) < 0.1) return;

    // Send command (throttled? or just send)
    // Invoking Tauri every frame (60fps) might be heavy, but let's try.
    invoke('send_robot_command', {
        id: parseInt(xboxId.value),
        team: parseInt(xboxTeam.value),
        vx: vx,
        vy: vy,
        omega: omega
    }).catch(console.error);
}


function loop() {
    pollGamepad();
    drawField();

    robots.blue.forEach(r => drawRobot(r.x, r.y, r.theta, 'blue', r.id));
    robots.yellow.forEach(r => drawRobot(r.x, r.y, r.theta, 'yellow', r.id));
    drawBall(ball.x, ball.y);

    // Vision connection timeout check
    if (Date.now() - lastVisionUpdate > 1000) {
        if (visionStatus) {
            visionStatus.textContent = "Disconnected";
            visionStatus.style.color = "#f00";
        }
    }

    requestAnimationFrame(loop);
}

// Start loop
loop();
