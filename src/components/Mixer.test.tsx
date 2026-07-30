import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import { Mixer } from './Mixer';
import { MidiLearnProvider } from '../context/MidiLearnContext';

// Mock Tauri invoke
const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((cmd: string, args: any) => mockInvoke(cmd, args)),
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn(async () => {
        return () => { }; // Unlisten function
    }),
}));

vi.mock('./EqCanvas', () => ({ EqCanvas: () => <div data-testid="eq-canvas" /> }));
vi.mock('./CompCanvas', () => ({ CompCanvas: () => <div data-testid="comp-canvas" /> }));
vi.mock('./TubeLimiterCanvas', () => ({ TubeLimiterCanvas: () => <div data-testid="tube-limiter" /> }));
vi.mock('./FilterCanvas', () => ({ FilterCanvas: () => <div data-testid="filter-canvas" /> }));
vi.mock('./ReverbCanvas', () => ({ ReverbCanvas: () => <div data-testid="reverb-canvas" /> }));
vi.mock('./DelayCanvas', () => ({ DelayCanvas: () => <div data-testid="delay-canvas" /> }));
vi.mock('./SaturationCanvas', () => ({ SaturationCanvas: () => <div data-testid="saturation-canvas" /> }));
vi.mock('./SynthCanvas', () => ({ SynthCanvas: () => <div data-testid="synth-canvas" /> }));
vi.mock('./MicroScope', () => ({ MicroScope: () => <div data-testid="micro-scope" /> }));
vi.mock('./LivingFader', () => ({ LivingFader: () => <div data-testid="living-fader" /> }));
vi.mock('./NanoEq', () => ({ NanoEq: () => <div data-testid="nano-eq" /> }));
vi.mock('./NanoComp', () => ({ NanoComp: () => <div data-testid="nano-comp" /> }));
vi.mock('./DriveKnob', () => ({ DriveKnob: () => <div data-testid="drive-knob" /> }));
vi.mock('./MagnetoSettings', () => ({ MagnetoSettings: () => <div data-testid="magneto-settings" /> }));

afterEach(cleanup);

describe('Mixer Component', () => {
    beforeEach(() => {
        mockInvoke.mockReset();
        // Default mocks
        mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === 'get_tracks') return [
                {
                    id: '1', name: 'Test Track',
                    volume: { id: 'vol1', value: 1.0, min_value: 0, max_value: 2, name: 'Volume' },
                    pan: { id: 'pan1', value: 0.0, min_value: -1, max_value: 1, name: 'Pan' },
                    width: { id: 'wid1', value: 1.0, min_value: 0, max_value: 2, name: 'Width' },
                    input_drive: { id: 'drv1', value: 0.0, min_value: 0, max_value: 1, name: 'Drive' },
                    eq_pre_dynamics: { id: 'ord1', value: 1.0, min_value: 0, max_value: 1, name: 'Order' },
                    is_muted: false, is_solo: false, is_armed: false, phase_inverted: false,
                    color: '#ff0000', effects: [],
                    console_eq: { id: 'eq1', name: 'EQ', parameters: [] },
                    console_comp: { id: 'comp1', name: 'Comp', parameters: [] },
                    peak_l: -10, rms_l: -12, peak_r: -10, rms_r: -12
                }
            ];
            if (cmd === 'get_master_meters') return { peak_l_db: -6, peak_r_db: -6, rms_l_db: -10, rms_r_db: -10 };
            return null;
        });
    });

    it('renders without crashing', async () => {
        render(
            <MidiLearnProvider>
                <Mixer />
            </MidiLearnProvider>
        );
        await waitFor(() => expect(screen.getByText('VIBE CONSOLE')).toBeInTheDocument());
    });

    it('displays tracks fetched from backend', async () => {
        render(
            <MidiLearnProvider>
                <Mixer />
            </MidiLearnProvider>
        );
        await waitFor(() => expect(screen.getByText('Test Track')).toBeInTheDocument());
    });

    it('sends mute command when Mute button clicked', async () => {
        render(
            <MidiLearnProvider>
                <Mixer />
            </MidiLearnProvider>
        );
        await waitFor(() => screen.getByText('Test Track'));

        const muteBtn = screen.getByText('M');
        fireEvent.click(muteBtn);

        expect(mockInvoke).toHaveBeenCalledWith('set_track_mute', { index: 0, muted: true });
    });

    it('adds an effect when FX button clicked', async () => {
        render(
            <MidiLearnProvider>
                <Mixer />
            </MidiLearnProvider>
        );
        await waitFor(() => screen.getByText('Test Track'));

        const reverbBtn = screen.getByText('RVB');
        fireEvent.click(reverbBtn);
        expect(mockInvoke).toHaveBeenCalledWith('add_effect', { index: 0, effectType: 'reverb' });
    });

    it('toggles solo, arm, and phase', async () => {
        render(
            <MidiLearnProvider>
                <Mixer />
            </MidiLearnProvider>
        );
        await waitFor(() => screen.getByText('Test Track'));

        const soloBtn = screen.getByText('S');
        fireEvent.click(soloBtn);
        expect(mockInvoke).toHaveBeenCalledWith('set_track_solo', { index: 0, solo: true });

        const armBtn = screen.getByText('R');
        fireEvent.click(armBtn);
        expect(mockInvoke).toHaveBeenCalledWith('set_track_arm', { index: 0, armed: true });

        const phaseBtn = screen.getByText('Ø');
        fireEvent.click(phaseBtn);
        expect(mockInvoke).toHaveBeenCalledWith('set_track_phase_invert', { index: 0, inverted: true });
    });

    it('adds new tracks and groups', async () => {
        render(
            <MidiLearnProvider>
                <Mixer />
            </MidiLearnProvider>
        );
        const addTrackBtn = screen.getByText('+ Audio Track');
        fireEvent.click(addTrackBtn);
        expect(mockInvoke).toHaveBeenCalledWith('add_track', expect.any(Object));

        const addGroupBtn = screen.getByText('+ Add Group');
        fireEvent.click(addGroupBtn);
        expect(mockInvoke).toHaveBeenCalledWith('add_bus', expect.any(Object));
    });

    it('adds various effects via buttons', async () => {
        render(
            <MidiLearnProvider>
                <Mixer />
            </MidiLearnProvider>
        );
        await waitFor(() => screen.getByText('Test Track'));

        const fxTypes = [
            { label: 'EQ', type: 'eq' },
            { label: 'SYN', type: 'vonesynth' },
            { label: 'DLY', type: 'delay' },
            { label: 'CMP', type: 'compressor' },
            { label: 'FIL', type: 'filter' },
            { label: 'SAT', type: 'saturation' },
        ];

        for (const fx of fxTypes) {
            const btn = screen.getAllByText(fx.label).find(el => el.classList.contains('btn-fx'));
            if (btn) {
                fireEvent.click(btn);
                expect(mockInvoke).toHaveBeenCalledWith('add_effect', { index: 0, effectType: fx.type });
            }
        }
    });

    it('opens effect editors on click', async () => {
        render(
            <MidiLearnProvider>
                <Mixer />
            </MidiLearnProvider>
        );
        await waitFor(() => screen.getByText('Test Track'));

        const eqSection = screen.getByTitle('EQ');
        fireEvent.click(eqSection);
        expect(screen.getByTestId('eq-canvas')).toBeInTheDocument();

        const compSection = screen.getByTitle('Compressor');
        fireEvent.click(compSection);
        expect(screen.getByTestId('comp-canvas')).toBeInTheDocument();
    });
});
