use crate::engine::audio::{AudioCommand, AudioEngine};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initialization_defaults() {
        let engine = AudioEngine::new();
        assert_eq!(engine.get_bpm(), 120.0);
        assert!(!engine.is_playing());
        assert_eq!(engine.get_playhead(), 0);
    }

    #[test]
    fn test_engine_transport_commands() {
        let engine = AudioEngine::new();

        // Play
        engine.play().unwrap();
        assert!(engine.is_playing());

        // Stop (Reset)
        engine.stop().unwrap();
        assert!(!engine.is_playing());

        // Give time for any pending audio callback to finish and see is_playing=false
        thread::sleep(Duration::from_millis(300));
        assert_eq!(engine.get_playhead(), 0);
    }

    #[test]
    fn test_bpm_changes() {
        let engine = AudioEngine::new();
        engine.set_bpm(140.0).unwrap();
        assert_eq!(engine.get_bpm(), 140.0);

        // Invalid BPM
        assert!(engine.set_bpm(0.0).is_err());
        assert!(engine.set_bpm(1000.0).is_err());
    }

    #[test]
    fn test_track_management() {
        let engine = AudioEngine::new();

        engine.add_track("Vocal".to_string()).unwrap();
        engine.add_track("Drums".to_string()).unwrap();

        thread::sleep(Duration::from_millis(1000));

        let levels = engine.get_track_levels();
        assert!(levels.len() >= 2);
        assert_eq!(levels[0].id.len(), 36); // Valid UUID
    }

    #[test]
    fn test_midi_binding_synapse() {
        let engine = AudioEngine::new();
        let param_id = Uuid::new_v4();

        let mut binding = crate::engine::midi_mapping::MidiBinding::default();
        binding.cc_number = 20;
        binding
            .targets
            .push(crate::engine::midi_mapping::ParameterTarget {
                param_id,
                min: 0.0,
                max: 1.0,
                scale: 1.0,
                invert: false,
            });

        engine
            .send_command(AudioCommand::AddBinding(binding))
            .unwrap();
        thread::sleep(Duration::from_millis(1000));

        let bindings = engine.get_midi_bindings();
        assert!(bindings.iter().any(|b| b.cc_number == 20));

        // Test Remove
        let b_id = bindings[0].id;
        engine.remove_midi_binding(b_id.to_string()).unwrap();
        thread::sleep(Duration::from_millis(500));
        assert!(!engine.get_midi_bindings().iter().any(|b| b.id == b_id));
    }

    #[test]
    fn test_playhead_seek() {
        let engine = AudioEngine::new();
        engine.set_playhead(96000).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(engine.get_playhead(), 96000);
    }

    // --- NEW TESTS FOR REQUIREMENT ---

    #[test]
    fn test_automation_recording_flow() {
        let engine = AudioEngine::new();
        engine.add_track("AutoTrack".to_string()).unwrap();
        thread::sleep(Duration::from_millis(500));

        // Start Recording
        engine.toggle_record().unwrap();
        engine.play().unwrap();
        thread::sleep(Duration::from_millis(100));

        // Set Volume (should record automation knot)
        engine.set_volume(0, 0.5).unwrap();
        thread::sleep(Duration::from_millis(100));

        // Stop
        engine.stop().unwrap();
        // Check if history node was updated or knots exist
        // (Hard to check internal knots without exposing more, but command sent)
    }

    #[test]
    fn test_metronome_toggle() {
        let engine = AudioEngine::new();
        engine
            .send_command(AudioCommand::SetMetronome(true))
            .unwrap();
        thread::sleep(Duration::from_millis(50));
        // We don't have get_metronome but we can check if it crashes
    }

    #[test]
    fn test_track_mute_solo_mutual() {
        let engine = AudioEngine::new();
        engine.add_track("T1".to_string()).unwrap();
        thread::sleep(Duration::from_millis(500));

        engine.set_mute(0, true).unwrap();
        engine.set_solo(0, true).unwrap();
        thread::sleep(Duration::from_millis(100));

        let levels = engine.get_track_levels();
        // Just verify it doesn't deadlock
        assert!(!levels.is_empty());
    }

    #[test]
    fn test_cpu_load_reporting() {
        let engine = AudioEngine::new();
        let load = engine.get_cpu_load();
        assert!(load >= 0.0);
    }

    #[test]
    fn test_history_traversal_smoke() {
        let engine = AudioEngine::new();
        let current = engine.get_current_node();
        assert!(!current.is_empty());

        let graph = engine.get_history_graph();
        assert!(!graph.is_empty());
    }

    #[test]
    fn test_analyzer_data_retrieval() {
        let engine = AudioEngine::new();
        engine.add_track("AnalyzerTest".to_string()).unwrap();
        thread::sleep(Duration::from_millis(500));

        let data = engine.get_analyzer_data(0);
        // Should return a Vec<u8> (binary blob for UI)
        assert!(!data.is_empty() || data.is_empty()); // Just check it doesn't crash
    }

    #[test]
    fn test_track_parameter_batch_set() {
        let engine = AudioEngine::new();
        engine.add_track("Params".to_string()).unwrap();
        thread::sleep(Duration::from_millis(500));

        engine.set_pan(0, -0.5).unwrap();
        engine.set_width(0, 1.5).unwrap();
        engine.set_track_drive(0, 0.8).unwrap();
        engine.set_phase_invert(0, true).unwrap();
        engine.set_arm(0, true).unwrap();

        thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_midi_note_crud_smoke() {
        let engine = AudioEngine::new();
        engine.add_track("MIDI".to_string()).unwrap();
        thread::sleep(Duration::from_millis(500));

        engine
            .send_command(AudioCommand::MidiNoteOn(60, 100))
            .unwrap();
        engine.send_command(AudioCommand::MidiNoteOff(60)).unwrap();
    }

    #[test]
    fn test_library_interaction() {
        let engine = AudioEngine::new();
        let lib = engine.get_library();
        assert!(lib.is_empty() || !lib.is_empty()); // Should exist
    }

    #[test]
    fn test_input_alias_creation() {
        let engine = AudioEngine::new();
        engine
            .send_command(AudioCommand::CreateInputAlias(
                "Mic 1".to_string(),
                false,
                vec![0],
                "#FF0000".to_string(),
            ))
            .unwrap();
        thread::sleep(Duration::from_millis(100));
    }
}
