import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { DriveKnob } from '../components/DriveKnob';
import React from 'react';

describe('DriveKnob', () => {
    it('renders with label and value', () => {
        render(<DriveKnob value={0.5} onChange={() => { }} />);
        expect(screen.getByText('50%')).toBeInTheDocument();
        expect(screen.getByText('DRIVE')).toBeInTheDocument();
    });

    it('triggers onChange when dragged', () => {
        const onChange = vi.fn();
        const { container } = render(<DriveKnob value={0.5} onChange={onChange} />);

        const knob = container.querySelector('.drive-knob')!;

        fireEvent.mouseDown(knob, { clientY: 100 });

        // Move up (increase)
        // dy = 20. speed = 0.005. change = 0.1
        fireEvent.mouseMove(window, { clientY: 80 });

        expect(onChange).toHaveBeenCalledWith(expect.closeTo(0.6, 2));

        fireEvent.mouseUp(window);
    });
});
