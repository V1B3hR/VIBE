import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./VirtualKeyboard.css";

const NOTES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const KEY_MAP: Record<string, number> = {
    'a': 60, 'w': 61, 's': 62, 'e': 63, 'd': 64, 'f': 65, 't': 66, 'g': 67, 'y': 68, 'h': 69, 'u': 70, 'j': 71, 'k': 72
};

export const VirtualKeyboard = () => {
    const handleNoteOn = async (note: number) => {
        try {
            await invoke("note_on", { note, velocity: 100 });
        } catch (e) {
            console.error(e);
        }
    };

    const handleNoteOff = async (note: number) => {
        try {
            await invoke("note_off", { note });
        } catch (e) {
            console.error(e);
        }
    };

    useEffect(() => {
        const activeKeys = new Set();

        const onKeyDown = (e: KeyboardEvent) => {
            if (activeKeys.has(e.key)) return;
            const note = KEY_MAP[e.key.toLowerCase()];
            if (note) {
                activeKeys.add(e.key);
                handleNoteOn(note);
            }
        };

        const onKeyUp = (e: KeyboardEvent) => {
            const note = KEY_MAP[e.key.toLowerCase()];
            if (note) {
                activeKeys.delete(e.key);
                handleNoteOff(note);
            }
        };

        window.addEventListener("keydown", onKeyDown);
        window.addEventListener("keyup", onKeyUp);
        return () => {
            window.removeEventListener("keydown", onKeyDown);
            window.removeEventListener("keyup", onKeyUp);
        };
    }, []);

    return (
        <div className="virtual-keyboard glass">
            <div className="keyboard-header">
                <span>V-ONE SYNTH VIRTUAL KEYS</span>
                <span className="hint">Use A-W-S-E-D... or click</span>
            </div>
            <div className="keys-container">
                {Array.from({ length: 13 }).map((_, i) => {
                    const noteNum = 60 + i;
                    const isBlack = NOTES[noteNum % 12].includes("#");
                    return (
                        <div
                            key={noteNum}
                            className={`key ${isBlack ? "black" : "white"}`}
                            onMouseDown={() => handleNoteOn(noteNum)}
                            onMouseUp={() => handleNoteOff(noteNum)}
                            onMouseLeave={() => handleNoteOff(noteNum)}
                        >
                            <span className="key-label">{NOTES[noteNum % 12]}</span>
                        </div>
                    );
                })}
            </div>
        </div>
    );
};
