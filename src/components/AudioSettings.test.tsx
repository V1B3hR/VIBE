import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import { AudioSettings } from './AudioSettings';

// Mock Tauri
const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((cmd: string, args: any) => mockInvoke(cmd, args)),
}));

describe('AudioSettings Component', () => {
    beforeEach(() => {
        mockInvoke.mockReset();
        mockInvoke.mockImplementation(async (cmd, args) => {
            if (cmd === 'get_audio_hosts') return ['WASAPI', 'ASIO', 'DirectSound'];
            if (cmd === 'get_buffer_sizes') return [64, 128, 256, 512, 1024];
            if (cmd === 'get_sample_rates') return [44100, 48000, 88200, 96000];
            if (cmd === 'get_current_audio_config') return {
                host_name: 'WASAPI',
                device_name: 'Default',
                sample_rate: 48000,
                buffer_size: 512,
                input_channels: 2,
                output_channels: 2
            };
            if (cmd === 'get_audio_devices') return [
                { id: '1', name: 'Speakers (Realtek)', host: 'WASAPI' }
            ];
            return null;
        });

        // Mock localStorage
        const store: Record<string, string> = {};
        vi.spyOn(Storage.prototype, 'getItem').mockImplementation((key) => store[key] || null);
        vi.spyOn(Storage.prototype, 'setItem').mockImplementation((key, value) => { store[key] = value; });

        // Mock window.alert
        window.alert = vi.fn();
    });

    afterEach(cleanup);

    it('renders and fetches audio configuration', async () => {
        render(<AudioSettings onClose={() => { }} />);

        await waitFor(() => expect(screen.getByText('⚙️ Audio Settings')).toBeInTheDocument());
        expect(mockInvoke).toHaveBeenCalledWith('get_audio_hosts', undefined);
        expect(mockInvoke).toHaveBeenCalledWith('get_current_audio_config', undefined);
    });

    it('updates latency calculation when buffer size changes', async () => {
        render(<AudioSettings onClose={() => { }} />);

        const bufferSelect = await screen.findByLabelText('Buffer Size');

        // 512 samples @ 48kHz = 10.67ms
        await waitFor(() => expect(screen.getByText('10.67ms')).toBeInTheDocument());

        fireEvent.change(bufferSelect, { target: { value: '256' } });

        // 256 samples @ 48kHz = 5.33ms
        await waitFor(() => expect(screen.getByText('5.33ms')).toBeInTheDocument());
    });

    it('calls set_audio_config when Apply clicked', async () => {
        const onClose = vi.fn();
        render(<AudioSettings onClose={onClose} />);

        await waitFor(() => screen.getByText('Apply Settings'));

        // Select a device first (the first one is auto-selected in useEffect)
        fireEvent.click(screen.getByText('Apply Settings'));

        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('set_audio_config', expect.any(Object)));
        expect(onClose).toHaveBeenCalled();
        expect(window.alert).toHaveBeenCalled();
    });

    it('updates UI scale via slider and persists to localStorage', async () => {
        render(<AudioSettings onClose={() => { }} />);

        const slider = screen.getByRole('slider');
        fireEvent.change(slider, { target: { value: '1.5' } });

        expect(localStorage.getItem('vibe-ui-scale')).toBe('1.5');
        expect(screen.getByText('UI Scale: 150%')).toBeInTheDocument();
    });
});
