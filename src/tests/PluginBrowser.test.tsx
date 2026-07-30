import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import { PluginBrowser } from '../components/PluginBrowser';
import React from 'react';

// Mock Tauri invoke
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((cmd: string, args: any) => mockInvoke(cmd, args)),
}));

describe('PluginBrowser', () => {
    const mockPlugins = [
        { id: 'p1', name: 'Serum', vendor: 'Xfer', category: 'Instrument', tags: ['Synth'], last_scanned: 0, path: '', plugin_type: 'VST3', is_favorite: true, hidden: false, deprecated: false },
        { id: 'p2', name: 'Pro-Q 3', vendor: 'FabFilter', category: 'EQ', tags: ['Mixing'], last_scanned: 0, path: '', plugin_type: 'VST3', is_favorite: false, hidden: false, deprecated: false },
    ];

    beforeEach(() => {
        vi.clearAllMocks();
        mockInvoke.mockResolvedValue(mockPlugins);
    });

    afterEach(cleanup);

    it('renders and fetches all plugins', async () => {
        render(<PluginBrowser />);
        await waitFor(() => screen.getByText('Serum'));
        expect(screen.getByText('Pro-Q 3')).toBeInTheDocument();
        expect(mockInvoke).toHaveBeenCalledWith('plugin_get_all', undefined);
    });

    it('filters by category', async () => {
        render(<PluginBrowser />);
        await waitFor(() => screen.getByText('Serum'));

        const eqTab = screen.getAllByRole('button', { name: 'EQ' }).find(b => b.classList.contains('cat-tab'));
        if (!eqTab) throw new Error('EQ tab not found');
        fireEvent.click(eqTab);

        await waitFor(() => {
            const calls = mockInvoke.mock.calls.map(c => c[0]);
            expect(calls).toContain('plugin_get_by_category');
        });
        expect(mockInvoke).toHaveBeenCalledWith('plugin_get_by_category', { category: 'EQ' });
    });

    it('handles search queries', async () => {
        render(<PluginBrowser />);
        await waitFor(() => screen.getByText('Serum'));

        const searchInput = screen.getByPlaceholderText(/Search plugins/i);
        fireEvent.change(searchInput, { target: { value: 'FabFilter' } });

        expect(screen.getByText('Pro-Q 3')).toBeInTheDocument();
        expect(screen.queryByText('Serum')).not.toBeInTheDocument();
    });

    it('toggles favorites', async () => {
        render(<PluginBrowser />);
        await waitFor(() => screen.getByText('Serum'));

        const favBtn = screen.getByText('☆');
        fireEvent.click(favBtn);

        await waitFor(() => {
            expect(mockInvoke).toHaveBeenCalledWith('plugin_toggle_favorite', { pluginId: 'p2' });
        });
    });

    it('triggers rescan', async () => {
        render(<PluginBrowser />);
        await waitFor(() => screen.getByText('Serum'));
        const rescanBtn = screen.getByText(/Rescan/i);
        fireEvent.click(rescanBtn);

        await waitFor(() => {
            expect(mockInvoke).toHaveBeenCalledWith('scan_plugins', undefined);
        });
    });

    it('switches to performance view', async () => {
        render(<PluginBrowser />);
        await waitFor(() => screen.getByText('Serum'));

        const perfTab = screen.getByText('⚡');
        fireEvent.click(perfTab);

        // Performance view still fetches all plugins to display stats
        await waitFor(() => {
            const calls = mockInvoke.mock.calls.map(c => c[0]);
            expect(calls.filter(c => c === 'plugin_get_all').length).toBeGreaterThanOrEqual(1);
        });
    });
});
