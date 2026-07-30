import { useState, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { safeInvoke } from '../../services/SafeInvoke';
import { useToast } from '../Toast';
import './LoadDialog.css';

interface RecentProject {
    path: string;
    name: string;
    lastOpened: number;
}

interface LoadDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onProjectLoaded: (path: string) => void;
}

export function LoadDialog({ isOpen, onClose, onProjectLoaded }: LoadDialogProps) {
    const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
    const [selectedProject, setSelectedProject] = useState<string>();
    const { showToast } = useToast();

    useEffect(() => {
        if (isOpen) {
            loadRecentProjects();
        }
    }, [isOpen]);

    const loadRecentProjects = () => {
        try {
            const stored = localStorage.getItem('vibe-recent-projects');
            if (stored) {
                const projects = JSON.parse(stored);
                setRecentProjects(projects);
            }
        } catch (error) {
            console.error('Failed to load recent projects:', error);
        }
    };

    const handleBrowse = async () => {
        try {
            const path = await open({
                filters: [{
                    name: 'VIBE Project',
                    extensions: ['vibe']
                }],
                multiple: false
            });

            if (path && typeof path === 'string') {
                await loadProject(path);
            }
        } catch (error) {
            console.error('Failed to open file dialog:', error);
        }
    };

    const loadProject = async (path: string) => {
        try {
            await safeInvoke('load_project_file', { path });

            // Update recent projects
            updateRecentProjects(path);

            showToast(`Project loaded: ${path.split(/[\\/]/).pop()}`, 'success');
            onProjectLoaded(path);
            onClose();
        } catch (error) {
            showToast(`Failed to load project: ${error}`, 'error');
        }
    };

    const updateRecentProjects = (path: string) => {
        try {
            const name = path.split(/[\\/]/).pop()?.replace('.vibe', '') || 'Untitled';
            const newRecent: RecentProject = {
                path,
                name,
                lastOpened: Date.now()
            };

            // Remove if already exists
            const filtered = recentProjects.filter(r => r.path !== path);

            // Add to beginning and limit to 10
            const updated = [newRecent, ...filtered].slice(0, 10);

            localStorage.setItem('vibe-recent-projects', JSON.stringify(updated));
            setRecentProjects(updated);
        } catch (error) {
            console.error('Failed to update recent projects:', error);
        }
    };

    const removeFromRecents = (path: string, e: React.MouseEvent) => {
        e.stopPropagation();

        const updated = recentProjects.filter(r => r.path !== path);
        localStorage.setItem('vibe-recent-projects', JSON.stringify(updated));
        setRecentProjects(updated);

        if (selectedProject === path) {
            setSelectedProject(undefined);
        }
    };

    const clearAllRecents = () => {
        if (confirm('Clear all recent projects?')) {
            localStorage.removeItem('vibe-recent-projects');
            setRecentProjects([]);
            setSelectedProject(undefined);
        }
    };

    const formatDate = (timestamp: number) => {
        const date = new Date(timestamp);
        const now = new Date();
        const diffMs = now.getTime() - date.getTime();
        const diffMins = Math.floor(diffMs / 60000);
        const diffHours = Math.floor(diffMs / 3600000);
        const diffDays = Math.floor(diffMs / 86400000);

        if (diffMins < 1) return 'Just now';
        if (diffMins < 60) return `${diffMins}m ago`;
        if (diffHours < 24) return `${diffHours}h ago`;
        if (diffDays < 7) return `${diffDays}d ago`;

        return date.toLocaleDateString();
    };

    if (!isOpen) return null;

    return (
        <div className="dialog-overlay" onClick={onClose}>
            <div className="dialog-content load-dialog" onClick={(e) => e.stopPropagation()}>
                <div className="dialog-header">
                    <h2>Open Project</h2>
                    <button className="dialog-close" onClick={onClose}>×</button>
                </div>

                <div className="dialog-body">
                    <div className="browse-section">
                        <button className="btn-browse-large" onClick={handleBrowse}>
                            📂 Browse for Project...
                        </button>
                    </div>

                    <div className="recent-section">
                        <div className="recent-header">
                            <h3>Recent Projects</h3>
                            {recentProjects.length > 0 && (
                                <button className="btn-clear-recents" onClick={clearAllRecents}>
                                    Clear All
                                </button>
                            )}
                        </div>

                        {recentProjects.length === 0 ? (
                            <div className="empty-state">
                                <p>No recent projects</p>
                                <span>Projects you open will appear here</span>
                            </div>
                        ) : (
                            <div className="recent-list">
                                {recentProjects.map((project) => (
                                    <div
                                        key={project.path}
                                        className={`recent-item ${selectedProject === project.path ? 'selected' : ''}`}
                                        onClick={() => setSelectedProject(project.path)}
                                        onDoubleClick={() => loadProject(project.path)}
                                    >
                                        <div className="recent-item-icon">🎵</div>
                                        <div className="recent-item-info">
                                            <div className="recent-item-name">{project.name}</div>
                                            <div className="recent-item-path">{project.path}</div>
                                            <div className="recent-item-date">{formatDate(project.lastOpened)}</div>
                                        </div>
                                        <button
                                            className="btn-remove-recent"
                                            onClick={(e) => removeFromRecents(project.path, e)}
                                            title="Remove from recents"
                                        >
                                            ×
                                        </button>
                                    </div>
                                ))}
                            </div>
                        )}
                    </div>
                </div>

                <div className="dialog-footer">
                    <button className="btn-secondary" onClick={onClose}>
                        Cancel
                    </button>
                    <button
                        className="btn-primary"
                        onClick={() => selectedProject && loadProject(selectedProject)}
                        disabled={!selectedProject}
                    >
                        Open Selected
                    </button>
                </div>
            </div>
        </div>
    );
}
