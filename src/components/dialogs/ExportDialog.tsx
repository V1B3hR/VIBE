import { useState, useEffect } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { safeInvoke } from '../../services/SafeInvoke';
import { useToast } from '../Toast';
import './ExportDialog.css';

interface ExportDialogProps {
    isOpen: boolean;
    onClose: () => void;
}

type ExportFormat = 'wav' | 'mp3' | 'flac' | 'aiff';
type BitDepth = 16 | 24 | 32;
type SampleRate = 44100 | 48000 | 96000 | 192000;
type ExportType = 'master' | 'tracks' | 'clips';
type DitherMode = 'None' | 'Triangular' | 'NoiseShaping';

export function ExportDialog({ isOpen, onClose }: ExportDialogProps) {
    const [exportType, setExportType] = useState<ExportType>('master');
    const [format, setFormat] = useState<ExportFormat>('wav');
    const [bitDepth, setBitDepth] = useState<BitDepth>(24);
    const [sampleRate, setSampleRate] = useState<SampleRate>(48000);
    const [outputPath, setOutputPath] = useState('');
    const [isExporting, setIsExporting] = useState(false);
    const [progress, setProgress] = useState(0);
    const [ditherMode, setDitherMode] = useState<DitherMode>('None');
    const [isNormalizeEnabled, setIsNormalizeEnabled] = useState(false);
    const [normalizeLufsTarget, setNormalizeLufsTarget] = useState(-14);
    const { showToast } = useToast();

    useEffect(() => {
        if (isOpen) {
            // Fetch current sample rate from audio config
            fetchAudioConfig();

            // Set up listener for export status
            const unlisten = listen('export_status', (event) => {
                const payload = event.payload as any;
                switch (payload.type) {
                    case 'progress':
                        setProgress(Math.round(payload.value * 100));
                        break;
                    case 'analysis':
                        console.log('Analysis result:', payload);
                        break;
                    case 'complete':
                        setProgress(100);
                        setIsExporting(false);
                        showToast(`Export completed: ${payload.path.split(/[\\/]/).pop()}`, 'success');
                        onClose();
                        break;
                    case 'error':
                        setIsExporting(false);
                        showToast(`Export failed: ${payload.message}`, 'error');
                        break;
                }
            });

            return () => {
                unlisten.then(f => f());
            };
        }
    }, [isOpen]);

    const fetchAudioConfig = async () => {
        try {
            const config = await safeInvoke<any>('get_audio_config');
            if (config && config.sample_rate) {
                setSampleRate(config.sample_rate as SampleRate);
            }
        } catch (error) {
            console.error('Failed to fetch audio config:', error);
        }
    };

    const handleBrowse = async () => {
        try {
            const extension = format === 'wav' ? 'wav' : format === 'mp3' ? 'mp3' : format === 'flac' ? 'flac' : 'aif';
            const path = await save({
                filters: [{
                    name: `${format.toUpperCase()} Audio`,
                    extensions: [extension]
                }],
                defaultPath: `export.${extension}`
            });

            if (path) {
                setOutputPath(path);
            }
        } catch (error) {
            console.error('Failed to open save dialog:', error);
        }
    };

    const handleExport = async () => {
        if (!outputPath) {
            showToast('Please select an output location', 'error');
            return;
        }

        setIsExporting(true);
        setProgress(0);

        try {
            // Map to backend RenderConfig
            const config = {
                output_path: outputPath,
                format: format === 'wav' ? 'Wav' : format === 'mp3' ? 'Mp3' : format === 'flac' ? 'Flac' : 'Aiff',
                sample_rate: sampleRate,
                bit_depth: bitDepth === 16 ? 'Integer16' : bitDepth === 24 ? 'Integer24' : 'Float32',
                dithering: ditherMode,
                normalize_lufs: isNormalizeEnabled ? normalizeLufsTarget : null,
                range: 'EntireProject',
                stem_export: [],
                dry_run: false,
                mp3_bitrate: 320
            };

            await safeInvoke('export_project', { config });
        } catch (error) {
            showToast(`Export failed to start: ${error}`, 'error');
            setIsExporting(false);
            setProgress(0);
        }
    };

    const getFormatDescription = () => {
        switch (format) {
            case 'wav':
                return 'Uncompressed, highest quality';
            case 'mp3':
                return 'Compressed, smaller file size';
            case 'flac':
                return 'Lossless compression';
            case 'aiff':
                return 'Apple Lossless standard';
        }
    };

    const getEstimatedSize = () => {
        // Rough estimation: 10MB per minute for 24-bit 48kHz stereo WAV
        const baseSize = 10;
        const bitDepthMultiplier = bitDepth / 24;
        const sampleRateMultiplier = sampleRate / 48000;
        const formatMultiplier = format === 'wav' ? 1 : format === 'mp3' ? 0.1 : 0.6;

        const estimatedMB = baseSize * bitDepthMultiplier * sampleRateMultiplier * formatMultiplier;
        return `~${estimatedMB.toFixed(1)} MB/min`;
    };

    if (!isOpen) return null;

    return (
        <div className="dialog-overlay" onClick={onClose}>
            <div className="dialog-content export-dialog" onClick={(e) => e.stopPropagation()}>
                <div className="dialog-header">
                    <h2>Export Audio</h2>
                    <button className="dialog-close" onClick={onClose}>×</button>
                </div>

                <div className="dialog-body">
                    {/* Export Type */}
                    <div className="form-group">
                        <label>Export Type</label>
                        <div className="export-type-selector">
                            <button
                                className={`export-type-btn ${exportType === 'master' ? 'active' : ''}`}
                                onClick={() => setExportType('master')}
                            >
                                <span className="type-icon">🎚️</span>
                                <span className="type-label">Master Mix</span>
                            </button>
                            <button
                                className={`export-type-btn ${exportType === 'tracks' ? 'active' : ''}`}
                                onClick={() => setExportType('tracks')}
                                disabled
                            >
                                <span className="type-icon">🎵</span>
                                <span className="type-label">Selected Tracks</span>
                            </button>
                            <button
                                className={`export-type-btn ${exportType === 'clips' ? 'active' : ''}`}
                                onClick={() => setExportType('clips')}
                                disabled
                            >
                                <span className="type-icon">✂️</span>
                                <span className="type-label">Selected Clips</span>
                            </button>
                        </div>
                    </div>

                    {/* Format */}
                    <div className="form-group">
                        <label>Format</label>
                        <div className="format-selector">
                            <button
                                className={`format-btn ${format === 'wav' ? 'active' : ''}`}
                                onClick={() => setFormat('wav')}
                            >
                                WAV
                            </button>
                            <button
                                className={`format-btn ${format === 'mp3' ? 'active' : ''}`}
                                onClick={() => setFormat('mp3')}
                            >
                                MP3
                            </button>
                            <button
                                className={`format-btn ${format === 'flac' ? 'active' : ''}`}
                                onClick={() => setFormat('flac')}
                            >
                                FLAC
                            </button>
                            <button
                                className={`format-btn ${format === 'aiff' ? 'active' : ''}`}
                                onClick={() => setFormat('aiff')}
                            >
                                AIFF
                            </button>
                        </div>
                        <div className="format-description">{getFormatDescription()}</div>
                    </div>

                    {/* Bit Depth */}
                    {(format === 'wav' || format === 'aiff' || format === 'flac') && (
                        <div className="form-group">
                            <label>Bit Depth</label>
                            <div className="bit-depth-selector">
                                <button
                                    className={`bit-depth-btn ${bitDepth === 16 ? 'active' : ''}`}
                                    onClick={() => setBitDepth(16)}
                                >
                                    16-bit
                                </button>
                                <button
                                    className={`bit-depth-btn ${bitDepth === 24 ? 'active' : ''}`}
                                    onClick={() => setBitDepth(24)}
                                >
                                    24-bit
                                </button>
                                {format !== 'flac' && (
                                    <button
                                        className={`bit-depth-btn ${bitDepth === 32 ? 'active' : ''}`}
                                        onClick={() => setBitDepth(32)}
                                    >
                                        32-bit
                                    </button>
                                )}
                            </div>
                        </div>
                    )}

                    {/* Sample Rate */}
                    <div className="form-group">
                        <label>Sample Rate</label>
                        <div className="sample-rate-selector">
                            <button
                                className={`sample-rate-btn ${sampleRate === 44100 ? 'active' : ''}`}
                                onClick={() => setSampleRate(44100)}
                            >
                                44.1 kHz
                            </button>
                            <button
                                className={`sample-rate-btn ${sampleRate === 48000 ? 'active' : ''}`}
                                onClick={() => setSampleRate(48000)}
                            >
                                48 kHz
                            </button>
                            <button
                                className={`sample-rate-btn ${sampleRate === 96000 ? 'active' : ''}`}
                                onClick={() => setSampleRate(96000)}
                            >
                                96 kHz
                            </button>
                            <button
                                className={`sample-rate-btn ${sampleRate === 192000 ? 'active' : ''}`}
                                onClick={() => setSampleRate(192000)}
                            >
                                192 kHz
                            </button>
                        </div>
                    </div>

                    {/* Dithering */}
                    {(format === 'wav' || format === 'aiff' || format === 'flac') && bitDepth < 32 && (
                        <div className="form-group">
                            <label>Dithering</label>
                            <div className="dither-selector">
                                <button
                                    className={`dither-btn ${ditherMode === 'None' ? 'active' : ''}`}
                                    onClick={() => setDitherMode('None')}
                                >
                                    None
                                </button>
                                <button
                                    className={`dither-btn ${ditherMode === 'Triangular' ? 'active' : ''}`}
                                    onClick={() => setDitherMode('Triangular')}
                                >
                                    Triangular
                                </button>
                                <button
                                    className={`dither-btn ${ditherMode === 'NoiseShaping' ? 'active' : ''}`}
                                    onClick={() => setDitherMode('NoiseShaping')}
                                >
                                    Noise Shaping
                                </button>
                            </div>
                        </div>
                    )}

                    {/* Normalization */}
                    <div className="form-group">
                        <div className="normalize-header">
                            <label className="checkbox-label">
                                <input
                                    type="checkbox"
                                    checked={isNormalizeEnabled}
                                    onChange={(e) => setIsNormalizeEnabled(e.target.checked)}
                                />
                                Normalize LUFS
                            </label>
                            {isNormalizeEnabled && (
                                <div className="normalize-input">
                                    <input
                                        type="number"
                                        value={normalizeLufsTarget}
                                        onChange={(e) => setNormalizeLufsTarget(parseFloat(e.target.value))}
                                        step="0.1"
                                        min="-30"
                                        max="-5"
                                    />
                                    <span>dB LUFS</span>
                                </div>
                            )}
                        </div>
                    </div>

                    {/* Output Path */}
                    <div className="form-group">
                        <label>Output Location</label>
                        <div className="path-selector">
                            <input
                                type="text"
                                value={outputPath}
                                readOnly
                                placeholder="Click Browse to select..."
                            />
                            <button className="btn-browse" onClick={handleBrowse}>
                                Browse...
                            </button>
                        </div>
                    </div>

                    {/* Export Info */}
                    <div className="export-info">
                        <div className="info-item">
                            <span className="info-label">Estimated Size:</span>
                            <span className="info-value">{getEstimatedSize()}</span>
                        </div>
                        <div className="info-item">
                            <span className="info-label">Quality:</span>
                            <span className="info-value">
                                {format.toUpperCase()} {format === 'wav' ? `${bitDepth}-bit` : ''} @ {sampleRate / 1000} kHz
                            </span>
                        </div>
                    </div>

                    {/* Progress Bar */}
                    {isExporting && (
                        <div className="export-progress">
                            <div className="progress-bar">
                                <div className="progress-fill" style={{ width: `${progress}%` }} />
                            </div>
                            <div className="progress-text">{progress}%</div>
                        </div>
                    )}
                </div>

                <div className="dialog-footer">
                    <button className="btn-secondary" onClick={onClose} disabled={isExporting}>
                        Cancel
                    </button>
                    <button
                        className="btn-primary"
                        onClick={handleExport}
                        disabled={!outputPath || isExporting}
                    >
                        {isExporting ? 'Exporting...' : 'Export'}
                    </button>
                </div>
            </div>
        </div>
    );
}
