import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MelSpectrogram } from './MelSpectrogram';

describe('MelSpectrogram Component', () => {
    const mockFrames = [
        { data: new Array(128).fill(-50), timestamp_samples: 0 },
        { data: new Array(128).fill(-30), timestamp_samples: 512 }
    ];

    beforeEach(() => {
        // Mock navigator.gpu
        (global.navigator as any).gpu = undefined;

        // Mock canvas methods
        HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue({
            fillRect: vi.fn(),
            clearRect: vi.fn(),
            getImageData: vi.fn(),
            putImageData: vi.fn(),
            createImageData: vi.fn().mockReturnValue({ data: new Uint8ClampedArray(100) }),
            beginPath: vi.fn(),
            arc: vi.fn(),
            fill: vi.fn(),
            drawImage: vi.fn(),
            imageSmoothingEnabled: true,
        });
    });

    it('renders loading state', () => {
        render(<MelSpectrogram frames={[]} width={800} height={300} loading={true} />);
        expect(screen.getByText(/ANALYZING SONIC ARCHITECTURE/i)).toBeInTheDocument();
    });

    it('renders container and canvas', () => {
        const { container } = render(
            <MelSpectrogram
                frames={mockFrames}
                width={800}
                height={300}
            />
        );
        expect(container.querySelector('.mel-spectrogram-container')).toBeInTheDocument();
        expect(container.querySelector('canvas')).toBeInTheDocument();
    });

    it('calculates frequency labels correctly', () => {
        render(
            <MelSpectrogram
                frames={mockFrames}
                width={800}
                height={300}
            />
        );
        // Should find some frequency labels (e.g. 1 kHz, 100 Hz etc as per our UI logic)
        expect(screen.getByText(/1 kHz/i)).toBeInTheDocument();
        expect(screen.getByText(/100 Hz/i)).toBeInTheDocument();
    });
});
