const { invoke } = window.__TAURI__.tauri;

export function initConfig() {
    const visionIp = document.getElementById('vision-ip');
    const visionPort = document.getElementById('vision-port');
    const btnReconnect = document.getElementById('btn-reconnect');
    const visionStatus = document.getElementById('vision-status');

    if (btnReconnect) {
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
    }

    // Radio Config
    const radioPortName = document.getElementById('radio-port-name');
    const radioBaudRate = document.getElementById('radio-baud-rate');
    const useRadio = document.getElementById('use-radio');
    const btnUpdateRadio = document.getElementById('btn-update-radio');

    if (btnUpdateRadio) {
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
    }

    const kfEnabled = document.getElementById('kf-enabled');
    const kfPNoiseP = document.getElementById('kf-pnoise-p');
    const kfPNoiseV = document.getElementById('kf-pnoise-v');
    const kfMNoise = document.getElementById('kf-mnoise');
    const btnUpdateKF = document.getElementById('btn-update-kf');

    if (btnUpdateKF) {
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
    }

    // --- Recording Logic ---
    const recFilename = document.getElementById('rec-filename');
    const btnRecordStart = document.getElementById('btn-record-start');
    const btnRecordStop = document.getElementById('btn-record-stop');
    const recStatus = document.getElementById('rec-status');

    if (btnRecordStart) {
        btnRecordStart.addEventListener('click', async () => {
            try {
                await invoke('start_recording', { filename: recFilename.value });
                if (recStatus) {
                    recStatus.textContent = "Recording...";
                    recStatus.style.color = "#0f0";
                }
                btnRecordStart.disabled = true;
                if (btnRecordStop) btnRecordStop.disabled = false;
            } catch (e) {
                console.error(e);
                alert(e);
            }
        });
    }

    if (btnRecordStop) {
        btnRecordStop.addEventListener('click', async () => {
            try {
                await invoke('stop_recording');
                if (recStatus) {
                    recStatus.textContent = "Saved";
                    recStatus.style.color = "#888";
                }
                if (btnRecordStart) btnRecordStart.disabled = false;
                btnRecordStop.disabled = true;
            } catch (e) {
                console.error(e);
            }
        });
    }

    // --- Collapsible Sections ---
    document.querySelectorAll('.section h4').forEach(h4 => {
        h4.addEventListener('click', () => {
            h4.parentElement.classList.toggle('collapsed');
        });
    });

    // --- Navbar Logic ---
    const btnConnection = document.getElementById('nav-btn-connection');
    const btnControl = document.getElementById('nav-btn-control');
    const panelsConnection = document.querySelectorAll('.panel-connection');
    const panelsControl = document.querySelectorAll('.panel-control');

    if (btnConnection && btnControl) {
        btnConnection.addEventListener('click', () => {
            btnConnection.classList.add('active');
            btnControl.classList.remove('active');
            panelsConnection.forEach(p => p.style.display = 'block');
            panelsControl.forEach(p => p.style.display = 'none');
        });

        btnControl.addEventListener('click', () => {
            btnControl.classList.add('active');
            btnConnection.classList.remove('active');
            panelsConnection.forEach(p => p.style.display = 'none');
            panelsControl.forEach(p => p.style.display = 'block');
        });
    }
}
