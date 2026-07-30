import sys
import json
import traceback
from music_theory_grove import MusicTheoryGrove

def main():
    # Initialize the brain
    # Point to the local data directory
    brain = MusicTheoryGrove(knowledge_path="data/knowledge/music_rules.json")
    
    # Ready signal (optional, but good for debugging)
    # sys.stderr.write("NeuralForest Service Ready\n")
    # sys.stderr.flush()

    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                break
            
            payload = json.loads(line)
            command = payload.get("command")
            data = payload.get("data", {})
            
            response = {}
            
            if command == "analyze_context":
                response = brain.analyze_audio_context(data)
            elif command == "generate_creative":
                response = brain.generate_creative_idea()
            elif command == "analyze_structure":
                density = data.get("density", [])
                response = brain.analyze_structure(density)
            elif command == "record_feedback":
                response = brain.record_feedback(data)
            elif command == "reset_memory":
                response = brain.reset_memory()
            else:
                response = {"error": f"Unknown command: {command}"}
            
            # Ensure response is a single line
            print(json.dumps(response))
            sys.stdout.flush()
            
        except Exception as e:
            error_msg = {
                "error": str(e),
                "traceback": traceback.format_exc()
            }
            print(json.dumps(error_msg))
            sys.stdout.flush()

if __name__ == "__main__":
    main()
