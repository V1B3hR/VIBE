import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./Mixer.css";
import { EqCanvas } from "./EqCanvas";
import { CompCanvas } from "./CompCanvas";
import { TubeLimiterCanvas } from "./TubeLimiterCanvas";
import { MagnetoSettings } from "./MagnetoSettings";
import { FilterCanvas } from "./FilterCanvas";
import { ReverbCanvas } from "./ReverbCanvas";
import { DelayCanvas } from "./DelayCanvas";
import { SaturationCanvas } from "./SaturationCanvas";
import { FrenzyCanvas } from "./FrenzyCanvas";
import { SynthCanvas } from "./SynthCanvas";
import { NanoEq } from "./NanoEq";
import { NanoComp } from "./NanoComp";
import { DriveKnob } from "./DriveKnob";
import { MicroScope } from "./MicroScope";
import { LivingFader } from "./LivingFader";
import { PluginRackUnit } from "./PluginRackUnit";
import { useMidiLearn } from "../context/MidiLearnContext";
import { ConvolutionReverbCanvas } from "./ConvolutionReverbCanvas";
import { MultibandDynamicsCanvas } from "./MultibandDynamicsCanvas";
import { SpectralGateCanvas } from "./SpectralGateCanvas";
import { StereoImagerCanvas } from "./StereoImagerCanvas";
// @ts-ignore
import * as ReactWindow from 'react-window';
const FixedSizeList = (ReactWindow as any).FixedSizeList || (ReactWindow as any).default?.FixedSizeList;

// @ts-ignore
import * as AutoSizerPkg from 'react-virtualized-auto-sizer';
const AutoSizer = (AutoSizerPkg as any).default || (AutoSizerPkg as any).AutoSizer;

interface Parameter {
  id: string;
  name: string;
  value: number;
  min_value: number;
  max_value: number;
}

interface Effect {
  id: string;
  name: string;
  is_bypassed: boolean;
  parameters: Parameter[];
}

interface Track {
  id: string;
  name: string;
  volume: Parameter;
  pan: Parameter;
  width: Parameter;
  is_muted: boolean;
  is_solo: boolean;
  is_armed: boolean;
  phase_inverted: boolean;
  input_source?: string;
  output_target?: string;
  color: string;
  bus_id?: string;
  effects: Effect[];
  console_eq: Effect;
  console_comp: Effect;
  eq_pre_dynamics: Parameter;
  peak_l: number;
  peak_r: number;
  rms_l: number;
  rms_r: number;
  lufs_l: number;
  lufs_r: number;
  input_drive: Parameter;
}

interface TrackLevel {
  id: string;
  peak_l: number;
  peak_r: number;
  rms_l: number;
  rms_r: number;
  true_peak_l: number;
  true_peak_r: number;
  lufs_momentary: number;
}

const RealTimeMeter = ({ peak, rms }: { peak: number, rms: number }) => {
  const [heldPeak, setHeldPeak] = useState(-144);
  const [isClipped, setIsClipped] = useState(false);

  useEffect(() => {
    if (peak > heldPeak) {
      setHeldPeak(peak);
    }
    if (peak > 0) {
      setIsClipped(true);
    }
  }, [peak]);

  const resetPeak = () => {
    setHeldPeak(-144);
    setIsClipped(false);
  }

  // Map dB to percentage (-60dB to 0dB)
  const mapDb = (db: number) => {
    if (db < -60) return 0;
    return Math.min(100, (db + 60) * (100 / 60));
  };

  const peakPct = mapDb(peak);
  const rmsPct = mapDb(rms);

  return (
    <div className="meter-v" onClick={resetPeak} title={`Peak: ${(heldPeak ?? -144).toFixed(1)} dB`}>
      {isClipped && <div className="meter-clip-ind" />}
      <div className="meter-val-readout">{(heldPeak ?? -144) > -60 ? (heldPeak ?? -144).toFixed(1) : '-inf'}</div>
      <div className="meter-bg">
        <div className="meter-fill rms" style={{ height: `${rmsPct}%` }}></div>
        <div className="meter-fill peak" style={{ height: `${peakPct}%` }}></div>
      </div>
    </div>
  );
};

export const Mixer = () => {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [editingEq, setEditingEq] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingComp, setEditingComp] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingTube, setEditingTube] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingFilter, setEditingFilter] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingReverb, setEditingReverb] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingDelay, setEditingDelay] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingSaturation, setEditingSaturation] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingSynth, setEditingSynth] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingFrenzy, setEditingFrenzy] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingConv, setEditingConv] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingMulti, setEditingMulti] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingSpec, setEditingSpec] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingImage, setEditingImage] = useState<{ trackId: number, processorId: string } | null>(null);
  const [editingMagneto, setEditingMagneto] = useState(false);
  const [masterMeters, setMasterMeters] = useState({
    peak_l_db: -60,
    peak_r_db: -60,
    rms_l_db: -60,
    rms_r_db: -60,
    lufs_integrated: -70,
    lufs_momentary: -70,
    lufs_short_term: -70,
    true_peak_l: -70,
    true_peak_r: -70
  });
  const [meterLevels, setMeterLevels] = useState<Record<string, TrackLevel>>({});
  const { isLearningMode, enterLearnMode, exitLearnMode, startLearningParameter, learningParamId } = useMidiLearn();

  const stripWidth = 153; // 140px width + roughly 13px spacing and border

  const fetchTracks = async () => {
    const trackList = await invoke<Track[]>("get_tracks");
    setTracks(trackList);
    // Initial sync of meters from full track list to avoid pop-in
    const levelMap = trackList.reduce((acc: Record<string, TrackLevel>, t: Track) => {
      acc[t.id] = {
        id: t.id,
        peak_l: t.peak_l || -60,
        peak_r: t.peak_r || -60,
        rms_l: t.rms_l || -60,
        rms_r: t.rms_r || -60,
        true_peak_l: -60,
        true_peak_r: -60,
        lufs_momentary: t.lufs_l || -70
      };
      return acc;
    }, {} as Record<string, TrackLevel>);
    setMeterLevels((prev: Record<string, TrackLevel>) => ({ ...prev, ...levelMap }));
  };

  const handleAddTrack = async () => {
    await invoke("add_track", { name: `Track ${tracks.length + 1}` });
    fetchTracks();
  };

  const handleVolumeChange = async (index: number, val: number) => {
    await invoke("set_track_volume", { index, volume: val });
    setTracks((prev: Track[]) => {
      const newTracks = [...prev];
      if (newTracks[index] && newTracks[index].volume) {
        newTracks[index].volume.value = val;
      }
      return newTracks;
    });
  };

  const handleMute = async (index: number, currentMuted: boolean) => {
    await invoke("set_track_mute", { index, muted: !currentMuted });
    setTracks(prev => {
      const newTracks = [...prev];
      if (newTracks[index]) {
        newTracks[index].is_muted = !currentMuted;
      }
      return newTracks;
    });
  };

  const handleSolo = async (index: number, currentSolo: boolean) => {
    await invoke("set_track_solo", { index, solo: !currentSolo });
    setTracks(prev => {
      const newTracks = [...prev];
      if (newTracks[index]) {
        newTracks[index].is_solo = !currentSolo;
      }
      return newTracks;
    });
  };

  const handlePhaseInvert = async (index: number, current: boolean) => {
    await invoke("set_track_phase_invert", { index, inverted: !current });
    fetchTracks();
  };

  const handleArm = async (index: number, current: boolean) => {
    await invoke("set_track_arm", { index, armed: !current });
    fetchTracks();
  };

  const handleToggleOrder = async (index: number, current: number) => {
    const newValue = current > 0.5 ? 0.0 : 1.0;
    const track = tracks[index];
    if (track.eq_pre_dynamics) {
      await invoke("set_parameter", { paramId: track.eq_pre_dynamics.id, value: newValue });
      fetchTracks();
    }
  };

  const handlePanChange = async (index: number, val: number) => {
    await invoke("set_track_pan", { index, pan: val });
    setTracks(prev => {
      const newTracks = [...prev];
      if (newTracks[index] && newTracks[index].pan) {
        newTracks[index].pan.value = val;
      }
      return newTracks;
    });
  };

  const handleWidthChange = async (index: number, val: number) => {
    await invoke("set_track_width", { index, width: val });
    setTracks(prev => {
      const newTracks = [...prev];
      if (newTracks[index] && newTracks[index].width) {
        newTracks[index].width.value = val;
      }
      return newTracks;
    });
  };

  const handleParameterChange = async (paramId: string, value: number) => {
    await invoke("set_parameter", { paramId, value });
    fetchTracks();
  };

  const handleAddEffect = async (index: number, type: string) => {
    await invoke("add_effect", { index, effectType: type });
    fetchTracks();
  };

  const handleDriveChange = async (index: number, val: number) => {
    await invoke("set_track_drive", { index, val });
    setTracks(prev => {
      const newTracks = [...prev];
      if (newTracks[index] && newTracks[index].input_drive) {
        newTracks[index].input_drive.value = val;
      }
      return newTracks;
    });
  };

  const handleBypassToggle = async (trackIndex: number, effectId: string, currentBypass: boolean) => {
    setTracks(prev => {
      const newTracks = [...prev];
      const track = newTracks[trackIndex];
      const effect = track.effects.find(e => e.id === effectId);
      if (effect) {
        effect.is_bypassed = !currentBypass;
      }
      return newTracks;
    });
    await invoke("set_effect_bypass", { trackIdx: trackIndex, processorId: effectId, bypass: !currentBypass });
    fetchTracks();
  };

  const handleEffectDragStart = (e: React.DragEvent, trackIndex: number, effectId: string, effectIndex: number) => {
    e.dataTransfer.setData("vibe/effect-id", effectId);
    e.dataTransfer.setData("vibe/track-index", trackIndex.toString());
    e.dataTransfer.setData("vibe/effect-index", effectIndex.toString());
    e.dataTransfer.effectAllowed = "move";
  };

  const handleEffectDropOnUnit = async (e: React.DragEvent, trackIndex: number, targetIndex: number) => {
    e.preventDefault();
    e.stopPropagation();

    const effectId = e.dataTransfer.getData("vibe/effect-id");
    const srcTrackIdxStr = e.dataTransfer.getData("vibe/track-index");

    if (effectId && srcTrackIdxStr) {
      const srcTrackIdx = parseInt(srcTrackIdxStr);
      if (srcTrackIdx === trackIndex) {
        await invoke("move_effect", { trackIdx: trackIndex, processorId: effectId, newIndex: targetIndex });
        fetchTracks();
      }
    } else {
      handlePluginDrop(e, trackIndex);
    }
  };

  const handleOpenEditor = (trackIdx: number, effectId: string, name: string) => {
    if (name === "Prisma EQ") {
      setEditingEq({ trackId: trackIdx, processorId: effectId });
    } else if (name === "Vibe Compressor") {
      setEditingComp({ trackId: trackIdx, processorId: effectId });
    } else if (name === "Magneto-Tube Limiter") {
      setEditingTube({ trackId: trackIdx, processorId: effectId });
    } else if (name === "VIBE Filter") {
      setEditingFilter({ trackId: trackIdx, processorId: effectId });
    } else if (name === "VIBE Reverb") {
      setEditingReverb({ trackId: trackIdx, processorId: effectId });
    } else if (name === "VIBE Delay") {
      setEditingDelay({ trackId: trackIdx, processorId: effectId });
    } else if (name === "VIBE Saturation") {
      setEditingSaturation({ trackId: trackIdx, processorId: effectId });
    } else if (name === "VOne Synth") {
      setEditingSynth({ trackId: trackIdx, processorId: effectId });
    } else if (name === "Frenzy Multiplier") {
      setEditingFrenzy({ trackId: trackIdx, processorId: effectId });
    } else if (name === "Convolution Reverb") {
      setEditingConv({ trackId: trackIdx, processorId: effectId });
    } else if (name === "Multiband Dynamics") {
      setEditingMulti({ trackId: trackIdx, processorId: effectId });
    } else if (name === "Spectral Gate") {
      setEditingSpec({ trackId: trackIdx, processorId: effectId });
    } else if (name === "Stereo Imager") {
      setEditingImage({ trackId: trackIdx, processorId: effectId });
    } else {
      invoke("open_plugin_editor", { trackIdx, pluginId: effectId });
    }
  };

  const handleRemoveEffect = async (trackIdx: number, effectId: string) => {
    await invoke("remove_effect", { trackIdx, processorId: effectId });
    fetchTracks();
  };

  const handlePluginDrop = async (e: React.DragEvent, trackIndex: number) => {
    e.preventDefault();
    const pluginPath = e.dataTransfer.getData("vibe/plugin-id");
    const effectId = e.dataTransfer.getData("vibe/effect-id");

    if (pluginPath) {
      await invoke("add_plugin_to_track", { trackIndex, pluginPath });
      fetchTracks();
    } else if (effectId) {
      const srcTrackIdxStr = e.dataTransfer.getData("vibe/track-index");
      const srcTrackIdx = srcTrackIdxStr ? parseInt(srcTrackIdxStr) : -1;
      if (srcTrackIdx === trackIndex) {
        const len = tracks[trackIndex].effects.length;
        await invoke("move_effect", { trackIdx: trackIndex, processorId: effectId, newIndex: len });
        fetchTracks();
      }
    }
  };

  useEffect(() => {
    fetchTracks();

    // Listen for project structure updates (add/remove track, etc.)
    const unlistenPromise = listen('project_updated', () => {
      fetchTracks();
    });

    // Fast Polling for Meters ONLY (30ms = ~30fps)
    const interval = setInterval(async () => {
      try {
        const levels = await invoke<TrackLevel[]>("get_track_levels");
        const levelMap = levels.reduce((acc: Record<string, TrackLevel>, l: TrackLevel) => {
          acc[l.id] = l;
          return acc;
        }, {} as Record<string, TrackLevel>);
        setMeterLevels(levelMap);

        const mm: any = await invoke("get_master_meters");
        setMasterMeters(mm);
      } catch (e) {
        console.error("Metering poll failed:", e);
      }
    }, 30);

    return () => {
      clearInterval(interval);
      unlistenPromise.then(unlisten => unlisten());
    };
  }, []);

  // Memoized row renderer — must NOT be inline or react-window re-mounts all visible
  // strips on every render (e.g., meter tick). Recreate only when track data changes.
  const TrackRenderer = useCallback(({ index, style }: any) => {
    const track = tracks[index];
    if (!track) return null;
    return (
      <div style={{ ...style, paddingRight: '8px', paddingBottom: '16px', boxSizing: 'border-box' }}>
        <div
          className="track-strip"
          style={{ "--track-color": track.color, width: "100%", height: "100%" } as React.CSSProperties}
        >
          <div
            className="fx-rack"
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => handlePluginDrop(e, index)}
          >
            <div style={{ display: 'flex', justifyContent: 'center', padding: '10px 0', borderBottom: '1px solid #333' }}>
              <DriveKnob
                value={track.input_drive?.value || 0}
                onChange={(v: number) => handleDriveChange(index, v)}
                size={40}
              />
            </div>

            <div className="fx-rack-divider">INSERTS</div>

            {track.effects.map((fx, fxIndex) => (
              <PluginRackUnit
                key={fx.id}
                effect={fx}
                trackIndex={index}
                effectIndex={fxIndex}
                onBypassToggle={handleBypassToggle}
                onParamChange={handleParameterChange}
                onOpenEditor={handleOpenEditor}
                onRemove={handleRemoveEffect}
                onDragStart={(e: React.DragEvent) => handleEffectDragStart(e, index, fx.id, fxIndex)}
                onDrop={(e: React.DragEvent) => handleEffectDropOnUnit(e, index, fxIndex)}
              />
            ))}
            {track.effects.length === 0 && <div className="fx-placeholder">Drop Plugins Here</div>}
          </div>

          <div style={{ display: 'flex', gap: '4px', padding: '6px', background: '#0a0a0a', borderTop: '1px solid #333' }}>
            <div title="EQ" onClick={() => setEditingEq({ trackId: index, processorId: track.console_eq.id })}>
              <NanoEq
                trackId={index}
                processorId={track.console_eq.id}
                params={track.console_eq.parameters}
                width={60} height={40}
              />
            </div>
            <button
              className="btn-order-toggle"
              onClick={() => handleToggleOrder(index, track.eq_pre_dynamics?.value)}
              title="Swap EQ/Dynamics"
              style={{ fontSize: '10px', padding: '2px', background: 'transparent', border: 'none', color: '#444', cursor: 'pointer' }}
            >
              ⇄
            </button>
            <div title="Compressor" onClick={() => setEditingComp({ trackId: index, processorId: track.console_comp.id })}>
              <NanoComp
                threshold={track.console_comp.parameters.find(p => p.name === "Threshold")?.value || -20}
                ratio={track.console_comp.parameters.find(p => p.name === "Ratio")?.value || 4}
                width={60} height={40}
              />
            </div>
          </div>

          <div style={{ display: 'flex', justifyContent: 'center', padding: '8px 0', background: '#111' }}>
            <MicroScope
              pan={track.pan?.value || 0}
              widthVal={track.width?.value || 1.0}
              onPanChange={(v: number) => handlePanChange(index, v)}
              onWidthChange={(v: number) => handleWidthChange(index, v)}
              size={60}
            />
          </div>

          <div className="v-slider-container" style={{ padding: '0 10px', height: '300px' }}>
            <div className="track-badges">
              <span className="badge-parallel">CORE-ISO</span>
            </div>
            <LivingFader
              value={track.volume?.value || 0.0}
              onChange={(v: number) => handleVolumeChange(index, v)}
              peakL={meterLevels[track.id]?.peak_l ?? -60}
              peakR={meterLevels[track.id]?.peak_r ?? -60}
              lufsM={meterLevels[track.id]?.lufs_momentary ?? -70}
              truePeakL={meterLevels[track.id]?.true_peak_l ?? -60}
              truePeakR={meterLevels[track.id]?.true_peak_r ?? -60}
              height={260}
            />
            <div className="db-value">{(track.volume?.value ?? -144) <= -60 ? '-inf' : (track.volume?.value ?? 0).toFixed(1)} dB</div>
          </div>

          <div className="track-buttons-mixer">
            <button
              className={`btn-mixer-mute ${track.is_muted ? 'active' : ''}`}
              onClick={() => handleMute(index, track.is_muted)}
              title="Mute"
            >M</button>
            <button
              className={`btn-mixer-solo ${track.is_solo ? 'active' : ''}`}
              onClick={() => handleSolo(index, track.is_solo || false)}
              title="Solo"
            >S</button>
            <button
              className={`btn-mixer-arm ${track.is_armed ? 'active' : ''}`}
              onClick={() => handleArm(index, track.is_armed || false)}
              title="Record Arm"
            >R</button>
            <button
              className={`btn-mixer-phase ${track.phase_inverted ? 'active' : ''}`}
              onClick={() => handlePhaseInvert(index, track.phase_inverted || false)}
              title="Phase Invert"
            >Ø</button>
          </div>

          <div className="track-name-container">
            <div className="track-name" title={track.name}>{track.name}</div>
            <div className="gold-accent-line"></div>
          </div>

          <div className="track-fx-buttons">
            <button className="btn-fx" onClick={() => handleAddEffect(index, "eq")}>EQ</button>
            <button className="btn-fx synth" onClick={() => handleAddEffect(index, "vonesynth")}>SYN</button>
            <button className="btn-fx delay" onClick={() => handleAddEffect(index, "delay")}>DLY</button>
            <button className="btn-fx reverb" onClick={() => handleAddEffect(index, "reverb")}>RVB</button>
            <button className="btn-fx compressor" onClick={() => handleAddEffect(index, "compressor")}>CMP</button>
            <button className="btn-fx eq" onClick={() => handleAddEffect(index, "eq")}>EQ</button>
            <button className="btn-fx filter" onClick={() => handleAddEffect(index, "filter")}>FIL</button>
            <button className="btn-fx tube" onClick={() => handleAddEffect(index, "saturation")}>SAT</button>
          </div>
        </div>
      </div>
    );
  }, [tracks, meterLevels,
    handleVolumeChange, handlePanChange, handleMute, handleSolo, handleArm,
    handleWidthChange, handlePhaseInvert, handleDriveChange, handleAddEffect,
    handleRemoveEffect, handleBypassToggle, handleEffectDropOnUnit,
    handleEffectDragStart, handleOpenEditor,
    setEditingEq, setEditingComp, setEditingTube, setEditingFilter,
    setEditingReverb, setEditingDelay, setEditingSaturation, setEditingSynth,
    setEditingFrenzy, setEditingConv, setEditingMulti, setEditingSpec, setEditingImage,
    isLearningMode, startLearningParameter, learningParamId, handlePluginDrop]);

  return (
    <div className="mixer-panel glass">
      {editingEq && (
        <div className="eq-modal-overlay" onClick={() => setEditingEq(null)}>
          <div className="eq-modal-content" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>Prisma EQ - Track {editingEq.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingEq(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <EqCanvas trackId={editingEq.trackId} processorId={editingEq.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingComp && (
        <div className="eq-modal-overlay" onClick={() => setEditingComp(null)}>
          <div className="eq-modal-content compressor-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>Vibe Compressor - Track {editingComp.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingComp(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <CompCanvas trackId={editingComp.trackId} processorId={editingComp.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingTube && (
        <div className="eq-modal-overlay" onClick={() => setEditingTube(null)}>
          <div className="eq-modal-content tube-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>Magneto-Tube - Track {editingTube.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingTube(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <TubeLimiterCanvas trackId={editingTube.trackId} processorId={editingTube.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingFilter && (
        <div className="eq-modal-overlay" onClick={() => setEditingFilter(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>VIBE Filter - Track {editingFilter.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingFilter(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <FilterCanvas trackId={editingFilter.trackId} processorId={editingFilter.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingReverb && (
        <div className="eq-modal-overlay" onClick={() => setEditingReverb(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>VIBE Reverb - Track {editingReverb.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingReverb(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <ReverbCanvas trackId={editingReverb.trackId} processorId={editingReverb.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingDelay && (
        <div className="eq-modal-overlay" onClick={() => setEditingDelay(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>VIBE Delay - Track {editingDelay.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingDelay(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <DelayCanvas trackId={editingDelay.trackId} processorId={editingDelay.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingSaturation && (
        <div className="eq-modal-overlay" onClick={() => setEditingSaturation(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>VIBE Saturation - Track {editingSaturation.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingSaturation(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <SaturationCanvas trackId={editingSaturation.trackId} processorId={editingSaturation.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingFrenzy && (
        <div className="eq-modal-overlay" onClick={() => setEditingFrenzy(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3 style={{ color: '#00ffff' }}>Frenzy Multiplier - Track {editingFrenzy.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingFrenzy(null)}>×</button>
            </div>
            <div className="eq-modal-body" style={{ padding: 0 }}>
              <FrenzyCanvas trackId={editingFrenzy.trackId} processorId={editingFrenzy.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingConv && (
        <div className="eq-modal-overlay" onClick={() => setEditingConv(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>Convolution Reverb - Track {editingConv.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingConv(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <ConvolutionReverbCanvas trackId={editingConv.trackId} processorId={editingConv.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingMulti && (
        <div className="eq-modal-overlay" onClick={() => setEditingMulti(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()} style={{ width: '700px' }}>
            <div className="eq-modal-header">
              <h3>Multiband Dynamics - Track {editingMulti.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingMulti(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <MultibandDynamicsCanvas trackId={editingMulti.trackId} processorId={editingMulti.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingSpec && (
        <div className="eq-modal-overlay" onClick={() => setEditingSpec(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>Spectral Gate - Track {editingSpec.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingSpec(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <SpectralGateCanvas trackId={editingSpec.trackId} processorId={editingSpec.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingImage && (
        <div className="eq-modal-overlay" onClick={() => setEditingImage(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>Stereo Imager - Track {editingImage.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingImage(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <StereoImagerCanvas trackId={editingImage.trackId} processorId={editingImage.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingSynth && (
        <div className="eq-modal-overlay" onClick={() => setEditingSynth(null)}>
          <div className="eq-modal-content filter-modal" onClick={e => e.stopPropagation()} style={{ width: '800px', maxWidth: '90vw' }}>
            <div className="eq-modal-header">
              <h3>V-One Instrument Editor - Track {editingSynth.trackId}</h3>
              <button className="btn-close-modal" onClick={() => setEditingSynth(null)}>×</button>
            </div>
            <div className="eq-modal-body">
              <SynthCanvas trackId={editingSynth.trackId} processorId={editingSynth.processorId} />
            </div>
          </div>
        </div>
      )}
      {editingMagneto && (
        <div className="eq-modal-overlay" onClick={() => setEditingMagneto(false)}>
          <div className="eq-modal-content magneto-modal" onClick={e => e.stopPropagation()}>
            <div className="eq-modal-header">
              <h3>VIBE Advanced Summing Setup</h3>
              <button className="btn-close-modal" onClick={() => setEditingMagneto(false)}>×</button>
            </div>
            <div className="eq-modal-body">
              <MagnetoSettings />
            </div>
          </div>
        </div>
      )}
      <div className="mixer-header">
        <div className="mixer-info">
          <h3>VIBE CONSOLE</h3>
          <div className="stability-hud">
            <span className="hud-label">ENGINE STABILITY:</span>
            <div className="hud-bar-bg">
              <div className="hud-bar-fill" style={{ width: '98%' }} />
            </div>
            <span className="hud-value">99.8%</span>
            <button className="hud-tag btn-magneto-open" onClick={() => setEditingMagneto(true)}>
              MAGNETO-GRAVITY SETUP
            </button>
            <button
              className={`hud-tag btn-magneto-open ${isLearningMode ? 'active' : ''}`}
              style={{ borderColor: isLearningMode ? '#ff00ff' : '', color: isLearningMode ? '#ff00ff' : '' }}
              onClick={() => isLearningMode ? exitLearnMode() : enterLearnMode()}
            >
              {isLearningMode ? 'EXIT SYNAPSE' : 'MIDI LEARN'}
            </button>
          </div>
        </div>
        <div className="mixer-controls">
          <button className="btn-add-track" onClick={handleAddTrack}>+ Audio Track</button>
          <button className="btn-add-bus" onClick={() => invoke("add_bus", { name: "New Group", color: "#ff4a4a" })}>+ Add Group</button>
        </div>
      </div>
      <div className="tracks-container" style={{ display: 'flex', flex: 1, gap: '16px', overflow: 'hidden' }}>
        <div style={{ flex: 1 }}>
          {/* @ts-ignore */}
          <AutoSizer>
            {({ height, width }: { height: number; width: number }) => (
              <FixedSizeList
                layout="horizontal"
                height={height}
                width={width}
                itemCount={tracks.length}
                itemSize={stripWidth}
                overscanCount={4}
              >
                {TrackRenderer}
              </FixedSizeList>
            )}
          </AutoSizer>
        </div>

        <div className="track-strip master-strip" style={{ flexShrink: 0, width: '140px' }}>
          <div className="fx-rack">
            <div className="fx-unit">
              <div className="fx-header">MASTER BUS</div>
              <div className="badge-parallel">MAYBACH WARMTH</div>
            </div>
          </div>

          <div className="spacer" style={{ flex: 1 }}></div>

          <div className="v-slider-container">
            <div className="track-badges">
              <span className="badge-parallel">STEREO-OUT</span>
              <span className="badge-bit">FLOAT-64</span>
            </div>
            <div className="fader-meter-row">
              <div className="master-lufs-display">
                <div className="lufs-item">
                  <span className="lufs-label">INT</span>
                  <span className="lufs-val">{(masterMeters.lufs_integrated ?? -70).toFixed(1)}</span>
                </div>
                <div className="lufs-item">
                  <span className="lufs-label">SHT</span>
                  <span className="lufs-val">{(masterMeters.lufs_short_term ?? -70).toFixed(1)}</span>
                </div>
              </div>
              <RealTimeMeter peak={masterMeters.peak_l_db} rms={masterMeters.rms_l_db} />
              <div className="master-fader-placeholder">
                <div className="fader-cap"></div>
              </div>
              <RealTimeMeter peak={masterMeters.peak_r_db} rms={masterMeters.rms_r_db} />
              <div className="master-tp-display">
                <div className="lufs-item">
                  <span className="lufs-label">TP L</span>
                  <span className={`lufs-val ${(masterMeters.true_peak_l ?? 0) > 0 ? 'clip' : ''}`}>{(masterMeters.true_peak_l ?? -70).toFixed(1)}</span>
                </div>
                <div className="lufs-item">
                  <span className="lufs-label">TP R</span>
                  <span className={`lufs-val ${(masterMeters.true_peak_r ?? 0) > 0 ? 'clip' : ''}`}>{(masterMeters.true_peak_r ?? -70).toFixed(1)}</span>
                </div>
              </div>
            </div>
            <div className="db-value">MASTER OUT</div>
          </div>

          <div className="track-name-container">
            <div className="track-name">MAIN OUT</div>
            <div className="gold-accent-line"></div>
          </div>
        </div>

        {tracks.length < 5 && Array.from({ length: 5 - tracks.length }).map((_, i) => (
          <div key={`empty-${i}`} className="track-strip empty">
            <div className="track-name">Empty Slot</div>
          </div>
        ))}
      </div>
    </div >
  );
};
