//! Viewer state: the simulator and the real-time playback clock.

use crate::field_view::FieldView;
use iced::widget::Canvas;
use iced::{time, Element, Length, Subscription};
use ssl_sim::{SimConfig, SimState, Simulator};
use ssl_sim_scenario::{Scenario, ScriptCursor, SCENARIO_ROBOT};
use std::time::{Duration, Instant};

/// How often the window advances the simulation.
const FRAME: Duration = Duration::from_millis(16);

pub struct App {
    scenario: Option<Scenario>,
    sim: Simulator,
    script: ScriptCursor,
    /// Simulated time owed to the real clock, s.
    backlog: f64,
    last_frame: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Frame(Instant),
}

impl App {
    pub fn new(scenario: Option<Scenario>) -> Self {
        let sim = match &scenario {
            Some(scenario) => Simulator::new(scenario.config.clone(), scenario.initial_state()),
            None => Simulator::new(SimConfig::default(), SimState::default()),
        };
        Self {
            scenario,
            sim,
            script: ScriptCursor::default(),
            backlog: 0.0,
            last_frame: None,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.running() {
            time::every(FRAME).map(Message::Frame)
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message) {
        let Message::Frame(now) = message;
        let Some(scenario) = &self.scenario else { return };

        if let Some(last) = self.last_frame {
            self.backlog += (now - last).as_secs_f64();
        }
        self.last_frame = Some(now);

        while self.backlog >= scenario.tick && self.sim.time() < scenario.duration {
            if let Some(command) = self.script.advance(scenario, self.sim.time()) {
                self.sim
                    .set_command(SCENARIO_ROBOT, command)
                    .expect("the scenario robot is never removed");
            }
            self.sim.step(scenario.tick);
            self.backlog -= scenario.tick;
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        Canvas::new(FieldView {
            state: self.sim.state(),
            config: self.sim.config(),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// A scenario is loaded and has time left to play.
    fn running(&self) -> bool {
        self.scenario
            .as_ref()
            .is_some_and(|scenario| self.sim.time() < scenario.duration)
    }
}
