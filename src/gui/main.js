// main.js - Entry point
import { initConfig } from './config.js';
import { initControl, pollManualControl, commandedVel, cmdId, cmdTeam, isActive } from './control.js';
import { initRendering, drawField, drawRobot, drawBall, ppsHistory, currentPPS, setCurrentPPS, drawPPSGraph } from './rendering.js';

const { listen } = window.__TAURI__.event;

const visionStatus = document.getElementById('vision-status');

let robots = {
    blue: [],
    yellow: []
};
let ball = { x: 0, y: 0 };
let lastVisionUpdate = 0;

// Initialize modules
initConfig();
initControl();
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

    const visualizeEl = document.getElementById('visualize-velocities');
    const visVels = visualizeEl && visualizeEl.checked;

    robots.blue.forEach(r => {
        let isControlled = isActive && cmdTeam === 0 && cmdId === r.id;
        drawRobot(r.x, r.y, r.theta, 'blue', r.id,
            visVels ? r.vx : 0,
            visVels ? r.vy : 0,
            (visVels && isControlled) ? commandedVel.vx : 0,
            (visVels && isControlled) ? commandedVel.vy : 0);
    });

    robots.yellow.forEach(r => {
        let isControlled = isActive && cmdTeam === 1 && cmdId === r.id;
        drawRobot(r.x, r.y, r.theta, 'yellow', r.id,
            visVels ? r.vx : 0,
            visVels ? r.vy : 0,
            (visVels && isControlled) ? commandedVel.vx : 0,
            (visVels && isControlled) ? commandedVel.vy : 0);
    });

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
