import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import { LivingFader } from '../components/LivingFader';
import React from 'react';

// Mock canvas context
const mockContext = {
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
    fillText: vi.fn(),
};

// @ts-ignore
HTMLCanvasElement.prototype.getContext = vi.fn(() => mockContext);

describe('LivingFader', () => {
    it('renders and responds to dragging', () => {
        const onChange = vi.fn();
        const { container } = render(
            <LivingFader
                value={0.0}
                onChange={onChange}
                peakL={-60}
                peakR={-60}
                lufsM={-60}
                truePeakL={-60}
                truePeakR={-60}
                height={300}
            />
        );

        const canvas = container.querySelector('canvas')!;

        // Start drag at 0dB (middle-ish)
        fireEvent.mouseDown(canvas, { clientY: 150 });

        // Move up (increase volume)
        // Sensitivity: 300px = 66dB. Moving 30px up should be +6.6dB, but it's clamped to 6.0 Max
        fireEvent.mouseMove(window, { clientY: 120 });
        expect(onChange).toHaveBeenCalledWith(expect.closeTo(6.0, 1));

        // Move down
        fireEvent.mouseMove(window, { clientY: 180 });
        expect(onChange).toHaveBeenLastCalledWith(expect.closeTo(-6.6, 0));

        fireEvent.mouseUp(window);
    });

    it('resets to 0.0 on double click', () => {
        const onChange = vi.fn();
        const { container } = render(
            <LivingFader
                value={-10.0}
                onChange={onChange}
                peakL={-60}
                peakR={-60}
                lufsM={-60}
                truePeakL={-60}
                truePeakR={-60}
            />
        );

        const canvas = container.querySelector('canvas')!;
        fireEvent.doubleClick(canvas);

        expect(onChange).toHaveBeenCalledWith(0.0);
    });
});
