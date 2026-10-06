use crate::config::FieldConfig;
use crate::math::Vec2;

/// Walls a circle touched, per axis: +1 the positive wall, -1 the negative
/// wall, 0 none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct WallContact {
    pub x: i8,
    pub y: i8,
}

impl WallContact {
    pub fn any(self) -> bool {
        self.x != 0 || self.y != 0
    }

    /// Replaces each velocity component that goes into a touched wall with
    /// `respond(component)`: e.g. 0 to stop, `-e * v` to bounce. Returns
    /// true if any component was replaced.
    pub fn apply(self, velocity: &mut Vec2, respond: impl Fn(f64) -> f64) -> bool {
        let mut hit = false;
        if self.x != 0 && velocity.x * f64::from(self.x) > 0.0 {
            velocity.x = respond(velocity.x);
            hit = true;
        }
        if self.y != 0 && velocity.y * f64::from(self.y) > 0.0 {
            velocity.y = respond(velocity.y);
            hit = true;
        }
        hit
    }
}

/// Pushes a circle of `radius` back inside the walls.
pub(crate) fn confine(field: &FieldConfig, position: &mut Vec2, radius: f64) -> WallContact {
    let limit_x = field.wall_x() - radius;
    let limit_y = field.wall_y() - radius;
    let mut contact = WallContact::default();

    if position.x > limit_x {
        position.x = limit_x;
        contact.x = 1;
    } else if position.x < -limit_x {
        position.x = -limit_x;
        contact.x = -1;
    }

    if position.y > limit_y {
        position.y = limit_y;
        contact.y = 1;
    } else if position.y < -limit_y {
        position.y = -limit_y;
        contact.y = -1;
    }

    contact
}
