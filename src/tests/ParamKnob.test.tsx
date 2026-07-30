import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ParamKnob } from '../components/ParamKnob';
import React from 'react';

describe('ParamKnob', () => {
    const defaultParam = {
        id: 'test-param',
        name: 'Gain',
        value: 0.5,
        min_value: 0.0,
        max_value: 1.0,
    };

    it('renders with name and value', () => {
        render(<ParamKnob param={defaultParam} onChange={() => { }} />);
        expect(screen.getByText('GAIN')).toBeInTheDocument();
        expect(screen.getByText('0.50')).toBeInTheDocument();
    });

    it('triggers onChange when dragged', () => {
        const onChange = vi.fn();
        render(<ParamKnob param={defaultParam} onChange={onChange} />);

        const knob = screen.getByTitle('Gain').querySelector('.param-knob')!;

        // Start drag
        fireEvent.mouseDown(knob, { clientY: 100 });

        // Move up (increase value)
        // Sensitivity is 200px for full range. Moving 20px up should be +0.1
        fireEvent.mouseMove(window, { clientY: 80 });

        expect(onChange).toHaveBeenCalledWith('test-param', expect.closeTo(0.6, 1));

        // Move down (decrease value)
        fireEvent.mouseMove(window, { clientY: 120 });
        expect(onChange).toHaveBeenCalledWith('test-param', expect.closeTo(0.4, 1));

        fireEvent.mouseUp(window);
    });

    it('respects min and max values', () => {
        const onChange = vi.fn();
        render(<ParamKnob param={defaultParam} onChange={onChange} />);

        const knob = screen.getByTitle('Gain').querySelector('.param-knob')!;

        fireEvent.mouseDown(knob, { clientY: 100 });

        // Move way up
        fireEvent.mouseMove(window, { clientY: -500 });
        expect(onChange).toHaveBeenLastCalledWith('test-param', 1.0);

        // Move way down
        fireEvent.mouseMove(window, { clientY: 1000 });
        expect(onChange).toHaveBeenLastCalledWith('test-param', 0.0);

        fireEvent.mouseUp(window);
    });
});
