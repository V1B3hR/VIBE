import { useState } from 'react';
import './FileMenu.css';

interface FileMenuProps {
    onNew: () => void;
    onOpen: () => void;
    onSave: () => void;
    onSaveAs: () => void;
    onExport: () => void;
    onRecentProject: (path: string) => void;
    recentProjects: Array<{ path: string; name: string }>;
}

export function FileMenu({
    onNew,
    onOpen,
    onSave,
    onSaveAs,
    onExport,
    onRecentProject,
    recentProjects,
}: FileMenuProps) {
    const [isOpen, setIsOpen] = useState(false);
    const [showRecents, setShowRecents] = useState(false);

    return (
        <div className="file-menu">
            <button
                className="file-menu-trigger"
                onClick={() => setIsOpen(!isOpen)}
                onBlur={() => setTimeout(() => setIsOpen(false), 200)}
            >
                File
            </button>

            {isOpen && (
                <div className="file-menu-dropdown">
                    <button className="menu-item" onClick={() => { onNew(); setIsOpen(false); }}>
                        <span className="menu-label">New Project</span>
                        <span className="menu-shortcut">Ctrl+N</span>
                    </button>

                    <button className="menu-item" onClick={() => { onOpen(); setIsOpen(false); }}>
                        <span className="menu-label">Open...</span>
                        <span className="menu-shortcut">Ctrl+O</span>
                    </button>

                    <div className="menu-separator" />

                    <button className="menu-item" onClick={() => { onSave(); setIsOpen(false); }}>
                        <span className="menu-label">Save</span>
                        <span className="menu-shortcut">Ctrl+S</span>
                    </button>

                    <button className="menu-item" onClick={() => { onSaveAs(); setIsOpen(false); }}>
                        <span className="menu-label">Save As...</span>
                        <span className="menu-shortcut">Ctrl+Shift+S</span>
                    </button>

                    <div className="menu-separator" />

                    <button className="menu-item" onClick={() => { onExport(); setIsOpen(false); }}>
                        <span className="menu-label">Export...</span>
                        <span className="menu-shortcut">Ctrl+E</span>
                    </button>

                    <div className="menu-separator" />

                    <div
                        className="menu-item submenu"
                        onMouseEnter={() => setShowRecents(true)}
                        onMouseLeave={() => setShowRecents(false)}
                    >
                        <span className="menu-label">Recent Projects</span>
                        <span className="menu-arrow">▶</span>

                        {showRecents && recentProjects.length > 0 && (
                            <div className="submenu-dropdown">
                                {recentProjects.map((project, idx) => (
                                    <button
                                        key={idx}
                                        className="menu-item"
                                        onClick={() => { onRecentProject(project.path); setIsOpen(false); }}
                                    >
                                        {project.name}
                                    </button>
                                ))}
                            </div>
                        )}

                        {showRecents && recentProjects.length === 0 && (
                            <div className="submenu-dropdown">
                                <div className="menu-item disabled">No recent projects</div>
                            </div>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
}
