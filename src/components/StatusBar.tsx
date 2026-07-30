import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import "./StatusBar.css";

export const StatusBar = () => {
    const [message, setMessage] = useState("Ready");
    const [type, setType] = useState<"info" | "success" | "warning" | "error">("info");
    const [lastAction, setLastAction] = useState<string | null>(null);

    useEffect(() => {
        const unlisten = listen<{ message: string; type: string }>("status_update", (event) => {
            setMessage(event.payload.message);
            setType(event.payload.type as any);

            // Auto-clear after 5 seconds if not error
            if (event.payload.type !== "error") {
                const timer = setTimeout(() => {
                    setMessage("Ready");
                    setType("info");
                }, 5000);
                return () => clearTimeout(timer);
            }
        });

        return () => {
            unlisten.then(f => f());
        };
    }, []);

    return (
        <div className={`status-bar ${type}`}>
            <div className="status-indicator" />
            <span className="status-message">{message}</span>
            <div className="status-right">
                <span className="status-sample-rate">48.0 kHz</span>
                <span className="status-bit-depth">24-bit</span>
                <span className="status-engine">VIBE ENGINE ACTIVE</span>
            </div>
        </div>
    );
};
