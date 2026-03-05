import { w as writable } from "./index.js";
writable(new Array(50).fill(0));
const controlTeam = writable(0);
const controlRobotId = writable(0);
const visualizeVelocities = writable(false);
const gamepadStatus = writable("No controller");
const gamepadConnected = writable(false);
const pathDrawMode = writable(false);
const pathTraceMode = writable(false);
const pathPoints = writable([]);
const VEL_CHART_SIZE = 600;
writable({
  vx: new Array(VEL_CHART_SIZE).fill(0),
  vy: new Array(VEL_CHART_SIZE).fill(0),
  omega: new Array(VEL_CHART_SIZE).fill(0)
});
const scriptPath = writable("");
const scriptStatus = writable("idle");
const POS_HISTORY_SIZE = 600;
writable({
  x: new Array(POS_HISTORY_SIZE).fill(0),
  y: new Array(POS_HISTORY_SIZE).fill(0),
  theta: new Array(POS_HISTORY_SIZE).fill(0)
});
export {
  pathPoints as a,
  scriptStatus as b,
  pathTraceMode as c,
  controlRobotId as d,
  controlTeam as e,
  gamepadStatus as f,
  gamepadConnected as g,
  pathDrawMode as p,
  scriptPath as s,
  visualizeVelocities as v
};
