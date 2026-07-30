#[cfg(test)]
mod tests {
    // use super::*;
    use crate::engine::io_manager::IoManager;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_input_alias_resolution() {
        // 1. Setup IoManager
        let io_manager = Arc::new(Mutex::new(IoManager::new(64)));

        // 2. Create an Input Alias (e.g., "Mic 1" on HW Channel 0)
        let alias_id = {
            let io = io_manager.lock().unwrap();
            io.create_input_alias("Mic 1".to_string(), false, vec![0], "#FF0000".to_string())
                .unwrap()
        };

        // 3. Simulate AudioCommand Handler Logic
        // We want to verify that given an Alias UUID, we can resolve it to Hardware Channels [0]

        let resolved_channels = {
            let io = io_manager.lock().unwrap();
            if let Some(alias) = io.get_input_alias(alias_id) {
                Some(alias.hardware_channels.clone())
            } else {
                None
            }
        };

        // 4. Assertions
        assert!(
            resolved_channels.is_some(),
            "Should resolve alias to channels"
        );
        assert_eq!(
            resolved_channels.unwrap(),
            vec![0],
            "Should map to hardware channel 0"
        );
    }

    #[test]
    fn test_mixing_logic_math() {
        // Simulate the mixing loop math to ensure f32->f64 casting and summation works correctly
        let frames = 4;
        let mut input_l = vec![0.0f64; frames];
        let mut input_r = vec![0.0f64; frames];

        // Mock Hardware Inputs (as f32, which comes from cpal)
        let hw_buf = vec![0.5f32; frames]; // Constant 0.5 signal
        let hardware_inputs = vec![hw_buf.clone(), hw_buf.clone()]; // Ch 0 and Ch 1

        // Mock Track configuration
        let track_channels = vec![0, 1]; // Stereo input from Ch 0 and 1

        // The Logic from run() loop
        if track_channels.len() >= 2 {
            let ch_l = track_channels[0];
            let ch_r = track_channels[1];

            let buf_l = &hardware_inputs[ch_l];
            let buf_r = &hardware_inputs[ch_r];

            for i in 0..frames {
                input_l[i] += buf_l[i] as f64;
                input_r[i] += buf_r[i] as f64;
            }
        }

        // Assertions
        for i in 0..frames {
            assert!(
                (input_l[i] - 0.5).abs() < 1e-9,
                "Left channel should be 0.5"
            );
            assert!(
                (input_r[i] - 0.5).abs() < 1e-9,
                "Right channel should be 0.5"
            );
        }
    }
    #[test]
    fn test_move_effect_logic() {
        // Validation of the index math used in AudioCommand::MoveEffect
        let mut processors: Vec<i32> = vec![0, 1, 2, 3, 4];

        // Case 1: Move Forward (1 -> 3)
        // User drags item 1 and drops it at position 3
        let from = 1;
        let to = 3;

        if from < processors.len() {
            let p = processors.remove(from); // [0, 2, 3, 4]
                                             // insert_at logic from audio.rs
            let insert_at = if to > from { to - 1 } else { to }; // 3-1 = 2
            if insert_at <= processors.len() {
                processors.insert(insert_at, p);
            }
        }
        // Result: [0, 2, 1, 3, 4] -> wait, original logic:
        // item 1 is '1'.
        // remove(1) -> [0, 2, 3, 4]. '1' is held.
        // to was 3. to > from. insert_at = 2.
        // insert(2, 1) into [0, 2, 3, 4].
        // index 0: 0
        // index 1: 2
        // index 2: 1  <-- inserted here
        // index 3: 3
        // index 4: 4
        // So [0, 2, 1, 3, 4].
        assert_eq!(processors, vec![0, 2, 1, 3, 4]);

        // Case 2: Move Backward (3 -> 1)
        // Current: [0, 2, 1, 3, 4]
        // Move item at index 3 (value 3) to index 1 (value 2)
        // Expected: [0, 3, 2, 1, 4]

        let from = 3;
        let to = 1;

        if from < processors.len() {
            let p = processors.remove(from); // [0, 2, 1, 4] (value 3 removed)
            let insert_at = if to > from { to - 1 } else { to }; // 1
            if insert_at <= processors.len() {
                processors.insert(insert_at, p);
            }
        }
        assert_eq!(processors, vec![0, 3, 2, 1, 4]);
    }
}
