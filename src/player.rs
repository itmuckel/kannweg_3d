use std::time::Instant;

#[derive(PartialEq)]
pub enum WalkState {
    Standing,
    Walking,
    Running,
}

pub struct Player {
    pub walk_state: WalkState,
    clock: Option<Instant>,
}

impl Player {
    pub const SPEED: f32 = 0.0175;
    pub const EXTRA_RUN_SPEED: f32 = 0.02;
    pub const MOUSE_SPEED: f32 = 0.15;

    pub fn run(&mut self) {
        if self.walk_state == WalkState::Walking {
            self.walk_state = WalkState::Running;
        }
    }

    pub fn walk(&mut self) {
        self.walk_state = WalkState::Walking;
        if self.clock.is_none() {
            self.clock = Some(Instant::now());
        }
    }

    pub fn should_play_step_sound(&mut self) -> bool {
        if let Some(clock) = self.clock {
            let time_between_steps = if self.walk_state == WalkState::Running {
                350u128
            } else {
                550u128
            };
            if clock.elapsed().as_millis() > time_between_steps {
                self.clock = Some(Instant::now());
                return true;
            }
        }
        return false;
    }

    pub fn stand(&mut self) {
        self.walk_state = WalkState::Standing;
        self.clock = None;
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            walk_state: WalkState::Standing,
            clock: None,
        }
    }
}
