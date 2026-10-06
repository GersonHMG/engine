//! Per-tick trajectory in the engine logger's CSV format, so a run can be
//! opened in the engine GUI's replay panel.
//!
//! One row per robot per frame (Team 0 = blue, 1 = yellow) plus one ball
//! row (RobotID = Team = -1).

use anyhow::{Context, Result};
use ssl_sim::{SimState, Team};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const HEADER: &str = "ElapsedTime,RobotID,Team,Vx_Command,Vy_Command,Angular_Command,\
                      Pos_X,Pos_Y,Orientation,Vx_Actual,Vy_Actual";

pub struct ReplayWriter {
    out: BufWriter<File>,
}

impl ReplayWriter {
    pub fn create(path: &Path) -> Result<Self> {
        let file = File::create(path).with_context(|| format!("cannot create {}", path.display()))?;
        let mut out = BufWriter::new(file);
        writeln!(out, "{HEADER}")?;
        Ok(Self { out })
    }

    pub fn write_frame(&mut self, state: &SimState) -> Result<()> {
        let (t, ball) = (state.time, &state.ball);
        for robot in state.robots() {
            let team = match robot.id.team {
                Team::Blue => 0,
                Team::Yellow => 1,
            };
            let command = &robot.command;
            writeln!(
                self.out,
                "{t:.3},{},{team},{},{},{},{},{},{},{},{}",
                robot.id.id,
                command.vt,
                command.vn,
                command.omega,
                robot.position.x,
                robot.position.y,
                robot.theta,
                robot.velocity.x,
                robot.velocity.y,
            )?;
        }
        writeln!(
            self.out,
            "{t:.3},-1,-1,0,0,0,{},{},0,{},{}",
            ball.position.x, ball.position.y, ball.velocity.x, ball.velocity.y,
        )?;
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.out.flush()?;
        Ok(())
    }
}
