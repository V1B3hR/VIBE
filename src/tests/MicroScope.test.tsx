import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import { MicroScope } from '../components/MicroScope';
import React from 'react';

// Mock canvas context
const mockContext = {
    clearRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    ellipse: vi.fn(),
    arc: vi.fn(),
    fill: vi.fn(),
    fillText: vi.fn(),
};

// @ts-ignore
HTMLCanvasElement.prototype.getContext = vi.fn(() => mockContext);

describe('MicroScope', () => {
    it('handles dragging for pan and width', () => {
        const onPanChange = vi.fn();
        const onWidthChange = vi.fn();
        const { container } = render(
            <MicroScope
                pan={0}
                widthVal={1.0}
                onPanChange={onPanChange}
                onWidthChange={onWidthChange}
            />
        );

        const canvas = container.querySelector('canvas')!;

        // Start drag at center
        fireEvent.mouseDown(canvas, { clientX: 100, clientY: 100 });

        // Move right and up (Increase Pan, Increase Width)
        // dx = 20, dy = 20 (up). 
        // panSense = 0.01 -> pan +0.2
        // widthSense = 0.01 -> width +0.2
        fireEvent.mouseMove(window, { clientX: 120, clientY: 80 });

        expect(onPanChange).toHaveBeenCalledWith(expect.closeTo(0.2, 2));
        expect(onWidthChange).toHaveBeenCalledWith(expect.closeTo(1.2, 2));

        fireEvent.mouseUp(window);
    });
});
