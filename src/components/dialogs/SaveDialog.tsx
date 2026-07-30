import { useState, useEffect } from 'react';
import { open, save } from '@tauri-apps/plugin-dialog';
import { safeInvoke } from '../../services/SafeInvoke';
import { useToast } from '../Toast';
import './SaveDialog.css';

interface SaveDialogProps {
    isOpen: boolean;
    onClose: () => void;
    currentPath?: string;
    isSaveAs?: boolean;
}

export function SaveDialog({ isOpen, onClose, currentPath, isSaveAs = false }: SaveDialogProps) {
    const [projectName, setProjectName] = useState('Untitled Project');
    const [author, setAuthor] = useState('');
    const [description, setDescription] = useState('');
    const [selectedPath, setSelectedPath] = useState(currentPath || '');
    const [bpm, setBpm] = useState(120);
    const [sampleRate, setSampleRate] = useState(48000);
    const { showToast } = useToast();

    useEffect(() => {
        if (isOpen) {
            // Fetch current project info
            fetchProjectInfo();
        }
    }, [isOpen]);

    const fetchProjectInfo = async () => {
        try {
            const transport = await safeInvoke<any>('get_transport_state');
            if (transport) {
                setBpm(transport.bpm || 120);
            }

            const config = await safeInvoke<any>('get_audio_config');
            if (config) {
                setSampleRate(config.sample_rate || 48000);
            }

            // Get author from localStorage
            const savedAuthor = localStorage.getItem('vibe-author');
            if (savedAuthor) {
                setAuthor(savedAuthor);
            }
        } catch (error) {
            console.error('Failed to fetch project info:', error);
        }
    };

    const handleBrowse = async () => {
        try {
            const path = await save({
                filters: [{
                    name: 'VIBE Project',
                    extensions: ['vibe']
                }],
                defaultPath: projectName + '.vibe'
            });

            if (path) {
                setSelectedPath(path);
                // Extract project name from path
                const name = path.split(/[\\/]/).pop()?.replace('.vibe', '') || 'Untitled Project';
                setProjectName(name);
            }
        } catch (error) {
            console.error('Failed to open save dialog:', error);
        }
    };

    const handleSave = async () => {
        if (!selectedPath) {
            showToast('Please select a save location', 'error');
            return;
        }

        try {
            // Ensure .vibe extension
            const savePath = selectedPath.endsWith('.vibe') ? selectedPath : selectedPath + '.vibe';

            // Save project
            await safeInvoke('save_project_file', { path: savePath });

            // Save metadata to localStorage
            localStorage.setItem('vibe-author', author);
            localStorage.setItem('lastProjectPath', savePath);

            // Add to recent projects
            addToRecentProjects(savePath, projectName);

            showToast(`Project saved: ${projectName}`, 'success');
            onClose();
        } catch (error) {
            showToast(`Failed to save project: ${error}`, 'error');
        }
    };

    const addToRecentProjects = (path: string, name: string) => {
        try {
            const recents = JSON.parse(localStorage.getItem('vibe-recent-projects') || '[]');
            const newRecent = {
                path,
                name,
                lastOpened: Date.now()
            };

            // Remove if already exists
            const filtered = recents.filter((r: any) => r.path !== path);

            // Add to beginning and limit to 10
            const updated = [newRecent, ...filtered].slice(0, 10);

            localStorage.setItem('vibe-recent-projects', JSON.stringify(updated));
        } catch (error) {
            console.error('Failed to update recent projects:', error);
        }
    };

    if (!isOpen) return null;

    return (
        <div className="dialog-overlay" onClick={onClose}>
            <div className="dialog-content save-dialog" onClick={(e) => e.stopPropagation()}>
                <div className="dialog-header">
                    <h2>{isSaveAs ? 'Save Project As' : 'Save Project'}</h2>
                    <button className="dialog-close" onClick={onClose}>×</button>
                </div>

                <div className="dialog-body">
                    <div className="form-group">
                        <label>Project Name</label>
                        <input
                            type="text"
                            value={projectName}
                            onChange={(e) => setProjectName(e.target.value)}
                            placeholder="My Awesome Track"
                            autoFocus
                        />
                    </div>

                    <div className="form-group">
                        <label>Author</label>
                        <input
                            type="text"
                            value={author}
                            onChange={(e) => setAuthor(e.target.value)}
                            placeholder="Your Name"
                        />
                    </div>

                    <div className="form-group">
                        <label>Description</label>
                        <textarea
                            value={description}
                            onChange={(e) => setDescription(e.target.value)}
                            placeholder="Optional project description..."
                            rows={3}
                        />
                    </div>

                    <div className="form-group">
                        <label>Save Location</label>
                        <div className="path-selector">
                            <input
                                type="text"
                                value={selectedPath}
                                readOnly
                                placeholder="Click Browse to select..."
                            />
                            <button className="btn-browse" onClick={handleBrowse}>
                                Browse...
                            </button>
                        </div>
                    </div>

                    <div className="project-info">
                        <div className="info-item">
                            <span className="info-label">BPM:</span>
                            <span className="info-value">{bpm}</span>
                        </div>
                        <div className="info-item">
                            <span className="info-label">Sample Rate:</span>
                            <span className="info-value">{sampleRate} Hz</span>
                        </div>
                    </div>
                </div>

                <div className="dialog-footer">
                    <button className="btn-secondary" onClick={onClose}>
                        Cancel
                    </button>
                    <button
                        className="btn-primary"
                        onClick={handleSave}
                        disabled={!selectedPath}
                    >
                        Save
                    </button>
                </div>
            </div>
        </div>
    );
}
