import { invoke } from "@tauri-apps/api/core";

class TelemetryService {
    private static instance: TelemetryService;

    private constructor() {
        this.init();
    }

    public static getInstance(): TelemetryService {
        if (!TelemetryService.instance) {
            TelemetryService.instance = new TelemetryService();
        }
        return TelemetryService.instance;
    }

    private init() {
        console.log("Telemetry initialized");
        window.addEventListener('click', this.handleClick.bind(this));
        window.addEventListener('keydown', this.handleKeydown.bind(this));

        // Log startup
        this.log("APP_START", `User Agent: ${navigator.userAgent}`);
    }

    private handleClick(event: MouseEvent) {
        const target = event.target as HTMLElement;
        const id = target.id || 'no-id';
        const className = target.className || 'no-class';
        const text = target.innerText?.substring(0, 20) || '';
        const coords = `${event.clientX},${event.clientY}`;

        this.log("CLICK", `Coords: ${coords} | Target: <${target.tagName.toLowerCase()} id="${id}" class="${className}"> "${text}"`);
    }

    private handleKeydown(event: KeyboardEvent) {
        this.log("KEYDOWN", `Key: ${event.key} | Code: ${event.code} | Modifiers: ${event.ctrlKey ? 'CTRL ' : ''}${event.shiftKey ? 'SHIFT ' : ''}${event.altKey ? 'ALT ' : ''}`);
    }

    public log(action: string, details: string = "") {
        // Fire and forget to backend
        try {
            invoke('log_frontend_action', { action, details });
        } catch (e) {
            console.error("Failed to verify log:", e);
        }
    }
}

export const Telemetry = TelemetryService.getInstance();
