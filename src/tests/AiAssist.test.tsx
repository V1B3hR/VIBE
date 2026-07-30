import { describe, it, expect, vi, beforeEach } from 'vitest';
import { aiAssistant, AiInsight } from '../services/AiAssistService';

// Mock Tauri invoke
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((cmd: string, args: any) => mockInvoke(cmd, args)),
}));

describe('AiAssistService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('broadcasts plugin tips correctly', async () => {
        const callback = vi.fn();
        aiAssistant.onInsight(callback);

        await aiAssistant.providePluginTip();

        expect(callback).toHaveBeenCalled();
        const insight = callback.mock.calls[0][0] as AiInsight;
        expect(insight.category).toBe('Plugin');
        expect(insight.message).toContain('Engineering Tip');
        expect(insight.pluginDetails).toBeDefined();
    });

    it('fetches deep knowledge from backend', async () => {
        const mockTip = {
            category: 'Mixing',
            title: 'Phase Alignment',
            body: 'Phase is everything in the low end.',
            importance: 0.8,
            reference_url: 'https://vibe.audio/edu/phase'
        };
        mockInvoke.mockResolvedValue(mockTip);

        const callback = vi.fn();
        aiAssistant.onInsight(callback);

        await aiAssistant.provideDeepKnowledge('Mixing');

        expect(mockInvoke).toHaveBeenCalledWith('get_assistant_knowledge_tip', { context: 'Mixing' });
        expect(callback).toHaveBeenCalled();
        const insight = callback.mock.calls[0][0] as AiInsight;
        expect(insight.category).toBe('Mixing');
        expect(insight.message).toContain('Deep Dive: Phase Alignment');
        expect(insight.severity).toBe(0.8);
    });

    it('triggers clipping warnings', async () => {
        const callback = vi.fn();
        aiAssistant.onInsight(callback);

        await aiAssistant.checkMastering(1.1);

        expect(callback).toHaveBeenCalled();
        const insight = callback.mock.calls[0][0] as AiInsight;
        expect(insight.category).toBe('Mastering');
        expect(insight.severity).toBe(0.9);
        expect(insight.targetElement).toBe('vibe-master-meters');
    });
});
