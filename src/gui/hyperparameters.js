const { appWindow } = window.__TAURI__.window;

const ids = [
    'ctrl-lat-kp', 'ctrl-lat-ki', 'ctrl-lat-kd', 'ctrl-speed-kp',
    'ctrl-head-kp', 'ctrl-target-speed', 'ctrl-lookahead',
    'ctrl-bb-amax', 'ctrl-bb-vmax', 'ctrl-pid-kp', 'ctrl-pid-maxv'
];

document.addEventListener('DOMContentLoaded', () => {
    // Determine which controller to show
    const params = new URLSearchParams(window.location.search);
    const controller = params.get('controller') || 'lookahead';

    // Update window title if possible, and header
    const titles = {
        'pid': 'PID Settings',
        'lookahead': 'LookAhead PID Settings',
        'bangbang': 'BangBangTrajectories Settings'
    };
    const titleText = titles[controller] || 'Controller Settings';
    document.title = titleText;
    const h3 = document.querySelector('h3');
    if (h3) h3.textContent = titleText;

    // Show/hide relevant rows
    document.querySelectorAll('.row').forEach(row => {
        if (row.dataset.controller && row.dataset.controller !== controller) {
            row.style.display = 'none';
        } else {
            row.style.display = 'flex';
        }
    });

    // Load existing values from localStorage
    ids.forEach(id => {
        const val = localStorage.getItem(id);
        if (val !== null) {
            const el = document.getElementById(id);
            if (el) el.value = val;
        }
    });

    // Save functionality
    document.getElementById('btn-save').addEventListener('click', () => {
        ids.forEach(id => {
            const el = document.getElementById(id);
            if (el) {
                localStorage.setItem(id, el.value);
            }
        });
        appWindow.close();
    });

    // Cancel functionality
    document.getElementById('btn-cancel').addEventListener('click', () => {
        appWindow.close();
    });
});
