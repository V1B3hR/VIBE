import os
import json
import time

class MusicTheoryGrove:
    def __init__(self, knowledge_path="data/knowledge/music_rules.json"):
        self.knowledge_path = knowledge_path
        self.short_term_memory_path = "data/knowledge/short_term.json"
        
        # Long Term Semantic Memory (Persistent behavior weights/rules)
        self.long_term = self._load_json(self.knowledge_path, default={
            "rules": {},
            "frustration_threshold": 0.8,
            "category_affinity": {
                "Mixing": 0.5,
                "Theory": 0.5,
                "Mastering": 0.5,
                "Groove": 0.5,
                "General": 0.5
            },
            "historical_metrics": {
                "avg_track_count": 0.0,
                "avg_plugin_count": 0.0,
                "avg_rms": -14.0,
                "session_points": 0
            },
            "total_suggestions": 0
        })

        # Short Term Episodic Memory (Current session activity)
        self.short_term = {
            "session_start": time.time(),
            "recent_actions": [],
            "current_mood": "Professional",
            "session_frustration": 0.0
        }
        self._save_short_term()

    def _load_json(self, path, default=None):
        if not os.path.exists(path):
            if default is not None:
                self._save_json(path, default)
            return default or {}
        try:
            with open(path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            return default or {}

    def _save_json(self, path, data):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=4)

    def _save_short_term(self):
        self._save_json(self.short_term_memory_path, self.short_term)

    def _save_long_term(self):
        self._save_json(self.knowledge_path, self.long_term)

    def record_feedback(self, data):
        """Learn from the user's acceptance or rejection of a suggestion."""
        action = data.get("action", "Unknown")
        accepted = data.get("accepted", False)
        
        # 1. Update Short-Term Episodic Memory
        actions_list = list(self.short_term.get("recent_actions", []))
        actions_list.append({
            "time": time.time(),
            "action": action,
            "accepted": accepted
        })
        # Keep only last 100 actions in memory
        if len(actions_list) > 100:
            actions_list.pop(0)

        self.short_term["recent_actions"] = actions_list

        current_frustration = float(self.short_term.get("session_frustration", 0.0))
        if accepted:
            current_frustration = max(0.0, current_frustration - 0.2)
        else:
            current_frustration = min(1.0, current_frustration + 0.1)

        self.short_term["session_frustration"] = current_frustration
        self.short_term["current_mood"] = "Supportive" if current_frustration > 0.5 else "Professional"
        self._save_short_term()

        # 2. Update Long-Term Semantic Memory
        self.long_term["total_suggestions"] += 1
        
        # Very basic learning: if action belongs to a category, adjust affinity
        category = "General"
        if action in ["ApplySmartEQ", "SidechainSuggestion", "BalanceTracks"]:
            category = "Mixing"
        elif action in ["ApplyLimiter"]:
            category = "Mastering"
        elif action in ["SetProjectScale", "InsertMidiProgression", "ModulateProject", "ApplyNegativeHarmony", "ApplyGenreTemplate"]:
            category = "Theory"
        elif action in ["ApplyGroove"]:
            category = "Groove"

        affinity = self.long_term["category_affinity"].get(category, 0.5)
        if accepted:
            # Shift affinity towards 1.0 slowly
            affinity = min(1.0, affinity + 0.05)
        else:
            # Shift affinity towards 0.0 slightly faster (frustration aversion)
            affinity = max(0.0, affinity - 0.08)
            
        self.long_term["category_affinity"][category] = affinity
        self._save_long_term()

        return {"status": "learned", "new_affinity": affinity, "mood": self.short_term["current_mood"]}

    def reset_memory(self):
        """Amnesia Button - resets all behavior data."""
        self.short_term = {
            "session_start": time.time(),
            "recent_actions": [],
            "current_mood": "Professional",
            "session_frustration": 0.0
        }
        self.long_term["category_affinity"] = {
            "Mixing": 0.5,
            "Theory": 0.5,
            "Mastering": 0.5,
            "Groove": 0.5,
            "General": 0.5
        }
        self._save_short_term()
        self._save_long_term()
        return {"status": "memory_wiped", "mood": "Professional"}

    def analyze_audio_context(self, data):
        # Proactive Apology Mechanism
        current_frustration = float(self.short_term.get("session_frustration", 0.0))
        if current_frustration >= 1.0:
            return {
                "text": "Widzę, że irytują Cię moje pomysły. Zamilknę na 15 minut i dam Ci popracować w ciszy. Powodzenia!",
                "action_type": None,
                "mood": "Apologetic",
                "emotion": "sad",
                "data": {"suppress_for": 900}
            }

        rms = float(data.get("rms_level", 0.0))
        clipping = bool(data.get("clipping_detected", False))
        spectral = float(data.get("spectral_balance", 0.5))
        scale = str(data.get("scale", "Unknown"))
        
        track_count = float(data.get("track_count", 0.0))
        plugin_count = float(data.get("plugin_count", 0.0))
        
        # --- Deep Learning Architecture: Update Historical Profile ---
        metrics = self.long_term.get("historical_metrics", {
            "avg_track_count": 0.0, "avg_plugin_count": 0.0, "avg_rms": -14.0, "session_points": 0
        })
        points = float(metrics.get("session_points", 0))
        
        # Rolling Average (Alpha = 0.05 for slow memory decay)
        alpha = 0.05
        if points == 0:
             metrics["avg_track_count"] = track_count
             metrics["avg_plugin_count"] = plugin_count
             metrics["avg_rms"] = rms
        else:
             metrics["avg_track_count"] = (1.0 - alpha) * float(metrics["avg_track_count"]) + (alpha * track_count)
             metrics["avg_plugin_count"] = (1.0 - alpha) * float(metrics["avg_plugin_count"]) + (alpha * plugin_count)
             metrics["avg_rms"] = (1.0 - alpha) * float(metrics["avg_rms"]) + (alpha * rms)
             
        metrics["session_points"] = points + 1
        self.long_term["historical_metrics"] = metrics
        self._save_long_term()
        
        # Find Deviation / "Behavioral Shifts" for Prompt Context
        historical_shifts = []
        if track_count > float(metrics["avg_track_count"]) + 15.0:
            historical_shifts.append("Producent używa znacznie więcej ścieżek niż jego standardowy workflow.")
        elif track_count > 0 and track_count < float(metrics["avg_track_count"]) - 10.0:
            historical_shifts.append("To bardzo minimalistyczny projekt w porównaniu do jego nawyków z przeszłości VIBE.")
            
        if plugin_count > float(metrics["avg_plugin_count"]) * 2.5 + 5:
            historical_shifts.append("Zauważono ekstremalnie duże użycie wtyczek (Podejrzenie: Przeładowany CPU lub skomplikowany mixdown).")

        shift_context = " ".join(historical_shifts) if historical_shifts else "Kontekst projektu w normie."

        # Build prompt for LLM
        sys_prompt = (
            "You are Kropelka, a world-class AI music producer assistant and analytical engine operating inside VIBE. "
            "You must respond entirely in Polish. Give EXACTLY 1 short, highly actionable, friendly sentence of advice. "
            "Do not use markdown, emojis, or introductory generic phrases. Be extremely specific to the provided telemetry context."
        )
        
        ctx_prompt = (
            f"VIBE Project Telemetry:\n"
            f"- RMS Level: {rms:.2f}dB (Historical Avg: {metrics['avg_rms']:.2f}dB)\n"
            f"- Master Clipping: {'Yes' if clipping else 'No'}\n"
            f"- Total Tracks: {int(track_count)}\n"
            f"- Total Active Plugins: {int(plugin_count)}\n"
            f"- Project Scale/Key: {scale}\n"
            f"- User Mood/State: {str(self.short_term.get('current_mood', 'Professional'))}\n"
            f"- Producer Historical Profile: {shift_context}\n\n"
            "Analyze the above statistics. Tell the producer exactly what they should focus on or be careful of right now."
        )
        
        llm_response, source = self._query_llm(sys_prompt, ctx_prompt)
        if llm_response:
            return {
                "text": f"[{source}] {llm_response}",
                "action_type": None,
                "mood": self.short_term["current_mood"],
                "emotion": "friendly",
                "data": {}
            }
        
        # Fallback to Rule-based system if both completely offline and Ollama is dead
        mood = self.short_term["current_mood"]
        base_text = "Analizuję Twój miks. Jestem do Twojej dyspozycji. (Brak połączenia z Ollama/Siecią)"
        
        if self.short_term["session_frustration"] > 0.6:
            base_text = "Weź głęboki oddech, świetnie Ci idzie."
            mood = "Supportive"
        elif self.long_term["category_affinity"].get("Mixing", 0.5) > 0.8:
            base_text = "Twoje umiejętności miksowania są dzisiaj znakomite!"
        
        return {
            "text": base_text,
            "action_type": None,
            "mood": mood,
            "emotion": "friendly",
            "data": {}
        }

    def generate_creative_idea(self):
        affinity = self.long_term["category_affinity"].get("Theory", 0.5)
        
        # Try LLM
        sys_prompt = "Jesteś asystentem AI Kropelka w DAW VIBE. Bądź zwięzły."
        ctx_prompt = "Zaproponuj jeden bardzo nieoczywisty, kreatywny pomysł na kompozycję lub sound design. Po polsku. 1 zdanie."
        llm_resp, src = self._query_llm(sys_prompt, ctx_prompt)
        
        if llm_resp:
             return {"text": f"[{src}] {llm_resp}", "action_type": "InsertMidiProgression", "mood": self.short_term["current_mood"]}

        if affinity < 0.3:
            return {"text": "Chciałem zasugerować trochę teorii muzyki, ale pozwolę Ci przejąć stery."}
            
        return {"text": "A gdybyśmy spróbowali zupełnie innej progresji akordów?", "action_type": "InsertMidiProgression", "mood": self.short_term["current_mood"]}

    def analyze_structure(self, density):
        sys_prompt = "Jesteś asystentem muzycznym. 1 zdanie analizy. Po polsku."
        ctx_prompt = f"Oto układ gęstości tracków (0-1): {density}. Co o tym sądzisz?"
        llm_resp, src = self._query_llm(sys_prompt, ctx_prompt)
        if llm_resp:
            return {"text": f"[{src}] {llm_resp}", "mood": self.short_term["current_mood"]}
            
        return {"text": "Struktura utworu ewoluuje.", "mood": self.short_term["current_mood"]}

    def _query_llm(self, sys_prompt, prompt_text):
        import urllib.request
        import urllib.error
        
        # 1. Try Local Ollama (Offline Priority)
        try:
            tags_req = urllib.request.Request("http://localhost:11434/api/tags")
            with urllib.request.urlopen(tags_req, timeout=1.0) as response:
                models_data = json.loads(response.read().decode())
                models = models_data.get("models", [])
                
                if models:
                    target_model = models[0]["name"]
                    for m in models:
                        if "llama" in m["name"] or "mistral" in m["name"] or "qwen" in m["name"]:
                            target_model = m["name"]
                            break
                            
                    payload = {
                        "model": target_model,
                        "prompt": f"{sys_prompt}\n\nContext:\n{prompt_text}",
                        "stream": False
                    }
                    
                    req = urllib.request.Request("http://localhost:11434/api/generate", 
                                                 data=json.dumps(payload).encode('utf-8'), 
                                                 headers={'Content-Type': 'application/json'})
                    with urllib.request.urlopen(req, timeout=5.0) as res:
                        result = json.loads(res.read().decode())
                        return result.get("response", "").strip(), "Ollama Offline"
        except Exception:
            pass
            
        # 2. Try Online (OpenAI API if key exists)
        import os
        openai_key = os.environ.get("OPENAI_API_KEY")
        if openai_key:
            try:
                req_data = {
                    "model": "gpt-4o-mini",
                    "messages": [
                        {"role": "system", "content": sys_prompt},
                        {"role": "user", "content": prompt_text}
                    ]
                }
                req = urllib.request.Request("https://api.openai.com/v1/chat/completions",
                                             data=json.dumps(req_data).encode("utf-8"),
                                             headers={
                                                 "Content-Type": "application/json",
                                                 "Authorization": f"Bearer {openai_key}"
                                             })
                with urllib.request.urlopen(req, timeout=3.0) as res:
                    result = json.loads(res.read().decode())
                    return result["choices"][0]["message"]["content"].strip(), "Cloud LLM (Online)"
            except Exception:
                pass
                
        # 3. Fallback
        return None, "Fallback"
