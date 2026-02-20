const { invoke } = window.__TAURI__.tauri;

const xboxActive = document.getElementById('xbox-active');
const xboxTeam = document.getElementById('xbox-team');
const xboxId = document.getElementById('xbox-id');
const xboxStatus = document.getElementById('xbox-status');
const manualControlMode = document.getElementById('manual-control-mode');
const manualVelScale = document.getElementById('manual-vel-scale');

let gamepadIndex = null;
let wasSendingManual = false;

export let commandedVel = { vx: 0, vy: 0 };
export let cmdId = 0;
export let cmdTeam = 0;
export let isActive = false;

const controlActiveIndicator = document.getElementById('control-active-indicator');

const keys = {
    'w': false,
    'a': false,
    's': false,
    'd': false,
    'q': false,
    'e': false
};

export function initControl() {
    window.addEventListener("gamepadconnected", (e) => {
        gamepadIndex = e.gamepad.index;
        if (xboxStatus) {
            xboxStatus.textContent = "Controller connected";
            xboxStatus.style.color = "#0f0";
        }
    });

    window.addEventListener("gamepaddisconnected", (e) => {
        if (gamepadIndex === e.gamepad.index) {
            gamepadIndex = null;
            if (xboxStatus) {
                xboxStatus.textContent = "No controller";
                xboxStatus.style.color = "#888";
            }
        }
    });

    window.addEventListener('keydown', (e) => {
        const key = e.key.toLowerCase();
        if (keys.hasOwnProperty(key)) {
            keys[key] = true;
        }
    });

    window.addEventListener('keyup', (e) => {
        const key = e.key.toLowerCase();
        if (keys.hasOwnProperty(key)) {
            keys[key] = false;
        }
    });
}

export function pollManualControl() {
    isActive = xboxActive && xboxActive.checked;

    if (controlActiveIndicator) {
        if (isActive) {
            controlActiveIndicator.classList.add('on');
        } else {
            controlActiveIndicator.classList.remove('on');
        }
    }

    if (!isActive) {
        commandedVel = { vx: 0, vy: 0 };
        return;
    }

    const mode = manualControlMode ? manualControlMode.value : 'xbox';
    let scale = manualVelScale ? parseFloat(manualVelScale.value) : 1.0;
    if (isNaN(scale)) scale = 1.0;

    let vx = 0;
    let vy = 0;
    let omega = 0;

    if (mode === 'xbox') {
        if (gamepadIndex !== null) {
            const gp = navigator.getGamepads()[gamepadIndex];
            if (gp) {
                // Axis 1 (Left Stick Y) -> Vx
                // Axis 0 (Left Stick X) -> Vy
                // Axis 2 (Right Stick X) -> Omega
                if (Math.abs(gp.axes[1]) > 0.1) vx = -gp.axes[1] * 2.0;
                if (Math.abs(gp.axes[0]) > 0.1) vy = -gp.axes[0] * 2.0;
                if (Math.abs(gp.axes[2]) > 0.1) omega = -gp.axes[2] * 4.0;
            }
        }
    } else if (mode === 'keyboard') {
        if (keys['w']) vx += 2.0;
        if (keys['s']) vx -= 2.0;
        if (keys['a']) vy += 2.0;
        if (keys['d']) vy -= 2.0;
        if (keys['q']) omega += 4.0;
        if (keys['e']) omega -= 4.0;
    }

    vx *= scale;
    vy *= scale;
    omega *= scale;

    if (Math.abs(vx) < 0.05 && Math.abs(vy) < 0.05 && Math.abs(omega) < 0.05) {
        vx = 0; vy = 0; omega = 0;
        if (!wasSendingManual) {
            commandedVel = { vx, vy };
            return; // already stopped
        }
        wasSendingManual = false;
    } else {
        wasSendingManual = true;
    }

    cmdId = parseInt(xboxId.value) || 0;
    cmdTeam = parseInt(xboxTeam.value) || 0;
    // Commanded velocities relative to robot frame (vx is forward, vy is lateral)
    commandedVel = { vx, vy };

    invoke('send_robot_command', {
        id: cmdId,
        team: cmdTeam,
        vx: vx,
        vy: vy,
        omega: omega
    }).catch(console.error);
}
