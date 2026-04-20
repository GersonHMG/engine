// packet_serializer.rs — Binary packet encoding for radio serial protocol
// Port of radio/packetserializer.cpp

use crate::types::RobotCommand;
use std::collections::HashMap;

/// Serialize robot commands into a byte buffer (5 bytes per robot).
pub fn serialize(command_map: &HashMap<i32, RobotCommand>, num_robots: usize) -> Vec<u8> {
    let mut packet = vec![0u8; num_robots * 5];

    for id in 0..num_robots as i32 {
        let mut bytes = [0u8; 5];

        if let Some(cmd) = command_map.get(&id) {
            let m = &cmd.motion;
            let k = &cmd.kicker;

            // Scale velocities by 100 to capture decimal part
            let mut vx = (m.vx * 100.0) as i32;
            let mut vy = (m.vy * 100.0) as i32;
            let mut vth = (m.angular * 100.0) as i32;
            let dribb = k.dribbler as i32;
            let kick: i32 = if k.kick_x { 1 } else { 0 };
            let callback: i32 = 0;

            // Clamp to [-511, 511]
            vx = vx.clamp(-511, 511);
            vy = vy.clamp(-511, 511);
            vth = vth.clamp(-511, 511);

            // Byte 0: ID(3) | dribbler(3) | kick(1) | callback(1)
            bytes[0] = ((id as u8 & 0x07) << 5)
                | ((dribb as u8 & 0x07) << 2)
                | ((kick as u8 & 0x01) << 1)
                | (callback as u8 & 0x01);

            // Byte 1: sign vX (1) | 7 bits abs(vX)
            bytes[1] = (if vx < 0 { 1u8 } else { 0u8 } << 7) | (vx.unsigned_abs() as u8 & 0x7F);

            // Byte 2: sign vY (1) | 7 bits abs(vY)
            bytes[2] = (if vy < 0 { 1u8 } else { 0u8 } << 7) | (vy.unsigned_abs() as u8 & 0x7F);

            // Byte 3: sign vTH (1) | 7 bits abs(vTH)
            bytes[3] =
                (if vth < 0 { 1u8 } else { 0u8 } << 7) | (vth.unsigned_abs() as u8 & 0x7F);

            // Byte 4: MSB vX (2) | MSB vY (2) | MSB vTH (4)
            bytes[4] = (((vx.unsigned_abs() >> 7) as u8 & 0x03) << 6)
                | (((vy.unsigned_abs() >> 7) as u8 & 0x03) << 4)
                | ((vth.unsigned_abs() >> 7) as u8 & 0x0F);
        }

        let offset = id as usize * 5;
        packet[offset..offset + 5].copy_from_slice(&bytes);
    }

    packet
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{KickerCommand, MotionCommand, RobotCommand};

    #[test]
    fn serialize_empty_map() {
        let map = HashMap::new();
        let result = serialize(&map, 6);
        assert_eq!(result.len(), 30);
        assert!(result.iter().all(|&b| b == 0));
    }

    #[test]
    fn serialize_single_command() {
        let mut map = HashMap::new();
        let mut cmd = RobotCommand::new(0, 0);
        cmd.motion = MotionCommand::new(0, 0, 1.5, -0.5);
        cmd.kicker = KickerCommand::new(0, 0);
        cmd.kicker.kick_x = true;
        map.insert(0, cmd);

        let result = serialize(&map, 6);
        assert_eq!(result.len(), 30);
        // Byte 0 should have kick bit set
        assert!(result[0] & 0x02 != 0, "Kick bit should be set");
    }
}
