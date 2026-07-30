import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import { Library } from './Library';

// Mock Tauri
const mockInvoke = vi.fn();
const mockOpen = vi.fn();
const mockListen = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((cmd: string, args: any) => mockInvoke(cmd, args)),
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn(async (event, handler) => {
        mockListen(event, handler);
        return () => { };
    }),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
    open: vi.fn(() => mockOpen()),
}));

// Mock ResizeObserver for VirtualScroll
class ResizeObserver {
    observe() { }
    unobserve() { }
    disconnect() { }
}
window.ResizeObserver = ResizeObserver;

describe('Library Component', () => {
    beforeEach(() => {
        mockInvoke.mockReset();
        mockOpen.mockReset();
        mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === 'get_library') return [
                { id: '1', name: 'Kick.wav', path: '/kick.wav', category: 'Drums', duration_seconds: 1.0, peaks: [0.5, 0.8, 0.4] }
            ];
            if (cmd === 'get_plugins' || cmd === 'plugin_get_all') return [
                { id: 'p1', name: 'Serum', path: '/serum.dll', vendor: 'Xfer', tags: [], plugin_type: 'VST3', category: 'Instrument', is_blacklisted: false, last_scanned: 0, is_favorite: false, hidden: false, deprecated: false }
            ];
            if (cmd === 'plugin_get_favorites') return [];
            if (cmd === 'plugin_get_recent') return [];
            return [];
        });
    });

    afterEach(cleanup);

    it('renders and fetches library items', async () => {
        render(<Library />);

        await waitFor(() => expect(screen.getByText('Kick.wav')).toBeInTheDocument());
        expect(mockInvoke).toHaveBeenCalledWith('get_library', undefined);
    });

    it('switches tabs between samples and plugins', async () => {
        render(<Library />);

        // Wait for initial load
        await waitFor(() => screen.getByText('Kick.wav'));

        const pluginsTab = screen.getByText('PLUGINS');
        fireEvent.click(pluginsTab);

        await waitFor(() => expect(screen.getByText('Serum')).toBeInTheDocument());
        expect(screen.queryByText('Kick.wav')).not.toBeInTheDocument();
    });

    it('triggers plugin scan', async () => {
        render(<Library />);

        const pluginsTab = screen.getByText('PLUGINS');
        fireEvent.click(pluginsTab);

        const scanBtn = await screen.findByTitle('Rescan Plugins');
        fireEvent.click(scanBtn);

        expect(mockInvoke).toHaveBeenCalledWith('scan_plugins', undefined);
    });

    it('toggles sync preview state', async () => {
        render(<Library />);

        const syncBtn = screen.getByTitle('Sync previews to Project BPM');
        expect(syncBtn.textContent).toContain('Sync On');

        fireEvent.click(syncBtn);
        expect(syncBtn.textContent).toContain('Sync Off');
    });

    it('creates an audio track when + Track clicked', async () => {
        render(<Library />);

        const addTrackBtn = screen.getByText('+ Track');
        fireEvent.click(addTrackBtn);

        expect(mockInvoke).toHaveBeenCalledWith('create_audio_track', { name: 'Audio Track' });
    });
});
