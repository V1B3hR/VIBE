import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock Tauri globals
(global.window as any).__TAURI_INTERNALS__ = {};

// Mock for @tauri-apps/api
Object.defineProperty(window, '__TAURI__', {
    value: {
        core: {
            invoke: vi.fn(async (cmd: string, args: any) => {
                console.log(`Mock Invoke (Global): ${cmd}`, args);
                if (cmd === 'get_tracks') return [];
                if (cmd === 'get_master_meters') return { peak_l_db: -60, peak_r_db: -60, rms_l_db: -60, rms_r_db: -60 };
                return null;
            })
        },
        event: {
            listen: vi.fn(async (_event: string, _callback: Function) => {
                return () => { }; // Unlisten function
            })
        }
    },
    writable: true
});

// Since @tauri-apps/api/v2 uses direct exports, we might need to mock them at the module level in tests too,
// but setupTests provides a fallback for components that might not be fully isolated.


// Polyfill for RequestAnimationFrame which might be missing in jsdom
global.requestAnimationFrame = (callback) => setTimeout(callback, 0);
global.cancelAnimationFrame = (id) => clearTimeout(id);

// Mock react-window and react-virtualized-auto-sizer for testing
vi.mock('react-window', () => {
    const React = require('react');
    return {
        FixedSizeList: ({ children, itemCount, itemSize }: any) => {
            const items = [];
            for (let i = 0; i < itemCount; i++) {
                items.push(children({ index: i, style: { width: itemSize } }));
            }
            return React.createElement('div', { 'data-testid': 'mock-fixed-size-list' }, items);
        }
    };
});

vi.mock('react-virtualized-auto-sizer', () => {
    return {
        default: ({ children }: any) => children({ height: 600, width: 800 }),
        AutoSizer: ({ children }: any) => children({ height: 600, width: 800 })
    };
});
