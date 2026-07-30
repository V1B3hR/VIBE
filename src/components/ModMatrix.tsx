
import React from 'react';
import './ModMatrix.css';

export type ModSource = 'None' | 'LFO' | 'Env1' | 'Env2' | 'Vel' | 'Key' | 'MacroX' | 'MacroY' | 'Seq';
export type ModDest = 'None' | 'Pitch1' | 'Shape1' | 'Pitch2' | 'Shape2' | 'Cutoff' | 'Res' | 'Drive' | 'LfoRate' | 'LfoAmt' | 'FxMixDelay' | 'FxMixReverb' | 'MasterVol' | 'FmAmt';

export interface ModSlot {
    src: ModSource;
    dest: ModDest;
    amount: number; // -1.0 to 1.0
    active: boolean;
}

interface ModMatrixProps {
    slots: ModSlot[];
    onChange: (index: number, slot: ModSlot) => void;
}

const SOURCES: ModSource[] = ['None', 'LFO', 'Env1', 'Env2', 'Vel', 'Key', 'MacroX', 'MacroY', 'Seq'];
const DESTINATIONS: ModDest[] = ['None', 'Pitch1', 'Shape1', 'Pitch2', 'Shape2', 'Cutoff', 'Res', 'Drive', 'LfoRate', 'LfoAmt', 'FxMixDelay', 'FxMixReverb', 'MasterVol', 'FmAmt'];

const ModMatrix: React.FC<ModMatrixProps> = ({ slots, onChange }) => {
    const handleSlotChange = (index: number, field: keyof ModSlot, value: any) => {
        const newSlot = { ...slots[index], [field]: value };
        onChange(index, newSlot);
    };

    return (
        <div className="mod-matrix glass">
            <div className="matrix-header">
                <span className="col-header">Source</span>
                <span className="col-header">Amount</span>
                <span className="col-header">Destination</span>
                <span className="col-header">On/Off</span>
            </div>
            <div className="matrix-rows">
                {slots.map((slot, i) => (
                    <div key={i} className={`matrix-row ${slot.active ? 'active' : ''}`}>

                        {/* Source Selector */}
                        <div className="matrix-cell">
                            <select
                                value={slot.src}
                                onChange={(e) => handleSlotChange(i, 'src', e.target.value as ModSource)}
                                className="matrix-dropdown"
                            >
                                {SOURCES.map(s => <option key={s} value={s}>{s}</option>)}
                            </select>
                        </div>

                        {/* Amount Slider */}
                        <div className="matrix-cell amount-cell">
                            <input
                                type="range"
                                min="-1"
                                max="1"
                                step="0.01"
                                value={slot.amount}
                                onChange={(e) => handleSlotChange(i, 'amount', parseFloat(e.target.value))}
                                className="matrix-slider"
                            />
                            <span className="amount-val">{(slot.amount * 100).toFixed(0)}%</span>
                        </div>

                        {/* Destination Selector */}
                        <div className="matrix-cell">
                            <select
                                value={slot.dest}
                                onChange={(e) => handleSlotChange(i, 'dest', e.target.value as ModDest)}
                                className="matrix-dropdown"
                            >
                                {DESTINATIONS.map(d => <option key={d} value={d}>{d}</option>)}
                            </select>
                        </div>

                        {/* Active Toggle */}
                        <div className="matrix-cell">
                            <button
                                className={`toggle-btn ${slot.active ? 'on' : 'off'}`}
                                onClick={() => handleSlotChange(i, 'active', !slot.active)}
                            >
                                {slot.active ? 'ON' : 'OFF'}
                            </button>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
};

export default ModMatrix;
