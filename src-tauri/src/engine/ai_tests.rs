#[cfg(test)]
mod tests {
    use crate::engine::kropelka_brain::KropelkaBrain;
    use crate::engine::neural_forest::NeuralForestBridge;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_brain_attachment() {
        let mut brain = KropelkaBrain::new();
        let bridge = Arc::new(Mutex::new(NeuralForestBridge::new(
            "python".to_string(),
            "script.py".to_string(),
        )));
        
        brain.attach_brain(bridge.clone());
        assert!(brain.forest_bridge.is_some());
    }

    #[test]
    fn test_learn_interaction_rejection() {
        let mut brain = KropelkaBrain::new();
        brain.persona.tone = "Chill".to_string();
        
        brain.learn_interaction("SuggestEffect", false);
        
        assert_eq!(brain.user_prefs.rejected_suggestions, 1);
        assert_eq!(brain.user_prefs.rejection_history.len(), 1);
        assert_eq!(brain.user_prefs.rejection_history[0].action_type, "SuggestEffect");
    }

    #[test]
    fn test_persona_adaptation() {
        let mut brain = KropelkaBrain::new();
        
        // Simulate many rejections
        for _ in 0..10 {
            brain.learn_interaction("test", false);
        }
        // Reset frustration level so it doesn't override to Supportive
        brain.user_prefs.recent_frustration_level = 0.0;
        brain.adapt_persona_from_prefs();
        
        // Rejection rate > 60% -> Assertive tone
        assert_eq!(brain.persona.tone, "Assertive");
        
        // Simulate many acceptances
        for _ in 0..41 {
            brain.learn_interaction("test", true);
        }
        // Reset frustration level
        brain.user_prefs.recent_frustration_level = 0.0;
        brain.adapt_persona_from_prefs();
        
        // Rejection rate < 20% -> Supportive tone
        assert_eq!(brain.persona.tone, "Supportive");
    }
}
