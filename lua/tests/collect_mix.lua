-- Training data for nn_sim: random excitation commands (see collect_dynamics.lua).
-- Change the seed for every recording (1, 2, 3, ...).
COLLECT = { mode = "mix", seed = 1, duration_min = 15 }
require("collect_dynamics")
