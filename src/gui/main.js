// main.js - Entry point
import { initConfig } from './config.js';
import { initControl, pollManualControl, commandedVel, cmdId, cmdTeam, isActive, pathPoints, initModal } from './control.js';
import { initRendering, drawField, drawRobot, drawRobotTrace, drawBall, drawPath, ppsHistory, currentPPS, setCurrentPPS, drawPPSGraph } from './rendering.js';

const { listen } = window.__TAURI__.event;

const visionStatus = document.getElementById('vision-status');

let robots = {
    blue: [],
    yellow: []
};
let ball = { x: 0, y: 0 };
let lastVisionUpdate = 0;
let robotTrace = [];
let lastTraceId = -1;
let lastTraceTeam = -1;
let lastTraceEnabled = false;

// Initialize modules
initConfig();
initControl();
initModal();
initRendering();

// Listen for vision updates
listen('vision-update', (event) => {
    const payload = event.payload;
    if (payload.robots_blue) robots.blue = payload.robots_blue;
    if (payload.robots_yellow) robots.yellow = payload.robots_yellow;
    if (payload.ball) ball = payload.ball;

    if (payload.pps !== undefined) {
        setCurrentPPS(payload.pps);
    }

    lastVisionUpdate = Date.now();
    if (visionStatus) {
        visionStatus.textContent = `Connected (${currentPPS} PPS)`;
        visionStatus.style.color = "#0f0";
    }
});

// Update PPS graph periodically
setInterval(() => {
    ppsHistory.shift();
    ppsHistory.push(currentPPS);
    drawPPSGraph();
}, 500);

// Main rendering and control loop
function loop() {
    pollManualControl();
    drawField();
    drawPath(pathPoints);

    const visualizeEl = document.getElementById('visualize-velocities');
    const visVels = visualizeEl && visualizeEl.checked;

    // Trace logic
    const pathTraceEl = document.getElementById('path-trace-mode');
    const isTraceEnabled = pathTraceEl && pathTraceEl.checked;
    const traceId = parseInt(document.getElementById('path-id')?.value || 0);
    const traceTeamStr = document.getElementById('path-team')?.value || "0";
    const traceTeam = parseInt(traceTeamStr);

    // Clear trace if disabled or changed target
    if (!isTraceEnabled || traceId !== lastTraceId || traceTeam !== lastTraceTeam) {
        robotTrace = [];
    }
    lastTraceEnabled = isTraceEnabled;
    lastTraceId = traceId;
    lastTraceTeam = traceTeam;

    robots.blue.forEach(r => {
        let isControlled = true; // Use server's command knowledge instead of purely client state
        drawRobot(r.x, r.y, r.theta, 'blue', r.id,
            visVels ? r.vx : 0,
            visVels ? r.vy : 0,
            (visVels && isControlled && r.cmd_vx) ? r.cmd_vx : 0,
            (visVels && isControlled && r.cmd_vy) ? r.cmd_vy : 0);

        if (isTraceEnabled && traceTeam === 0 && r.id === traceId) {
            robotTrace.push({ x: r.x, y: r.y });
        }
    });

    robots.yellow.forEach(r => {
        let isControlled = true;
        drawRobot(r.x, r.y, r.theta, 'yellow', r.id,
            visVels ? r.vx : 0,
            visVels ? r.vy : 0,
            (visVels && isControlled && r.cmd_vx) ? r.cmd_vx : 0,
            (visVels && isControlled && r.cmd_vy) ? r.cmd_vy : 0);

        if (isTraceEnabled && traceTeam === 1 && r.id === traceId) {
            robotTrace.push({ x: r.x, y: r.y });
        }
    });

    if (isTraceEnabled && robotTrace.length > 0) {
        drawRobotTrace(robotTrace);
    }

    drawBall(ball.x, ball.y);

    // Vision connection timeout check
    if (Date.now() - lastVisionUpdate > 1000) {
        if (visionStatus) {
            visionStatus.textContent = "Disconnected";
            visionStatus.style.color = "#f00";
        }

        // Draw gray overlay on canvas
        const canvas = document.getElementById('fieldCanvas');
        if (canvas) {
            const ctx = canvas.getContext('2d');
            ctx.fillStyle = 'rgba(128, 128, 128, 0.5)';
            ctx.fillRect(0, 0, canvas.width, canvas.height);

            ctx.fillStyle = 'white';
            ctx.font = '30px Arial';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText('No Vision Connected', canvas.width / 2, canvas.height / 2);
        }
    }

    requestAnimationFrame(loop);
}

// Start loop
loop();
