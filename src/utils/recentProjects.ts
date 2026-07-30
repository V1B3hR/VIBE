/**
 * Recent Projects Manager
 * 
 * Manages the list of recently opened projects with localStorage persistence.
 * Maximum 10 recent projects are stored.
 */

export interface RecentProject {
    path: string;
    name: string;
    lastOpened: number;
    trackCount?: number;
    duration?: number;
}

const STORAGE_KEY = 'vibe-recent-projects';
const MAX_RECENT_PROJECTS = 10;

/**
 * Get all recent projects, sorted by last opened (newest first)
 */
export function getRecentProjects(): RecentProject[] {
    try {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (!stored) return [];

        const projects = JSON.parse(stored) as RecentProject[];
        return projects.sort((a, b) => b.lastOpened - a.lastOpened);
    } catch (error) {
        console.error('Failed to load recent projects:', error);
        return [];
    }
}

/**
 * Add or update a project in the recent list
 */
export function addToRecentProjects(path: string, name?: string): void {
    try {
        const projects = getRecentProjects();

        // Extract name from path if not provided
        const projectName = name || path.split(/[\\/]/).pop()?.replace('.vibe', '') || 'Untitled Project';

        // Create new recent project entry
        const newProject: RecentProject = {
            path,
            name: projectName,
            lastOpened: Date.now()
        };

        // Remove if already exists
        const filtered = projects.filter(p => p.path !== path);

        // Add to beginning and limit to MAX_RECENT_PROJECTS
        const updated = [newProject, ...filtered].slice(0, MAX_RECENT_PROJECTS);

        localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
    } catch (error) {
        console.error('Failed to add to recent projects:', error);
    }
}

/**
 * Remove a project from the recent list
 */
export function removeFromRecentProjects(path: string): void {
    try {
        const projects = getRecentProjects();
        const filtered = projects.filter(p => p.path !== path);
        localStorage.setItem(STORAGE_KEY, JSON.stringify(filtered));
    } catch (error) {
        console.error('Failed to remove from recent projects:', error);
    }
}

/**
 * Clear all recent projects
 */
export function clearRecentProjects(): void {
    try {
        localStorage.removeItem(STORAGE_KEY);
    } catch (error) {
        console.error('Failed to clear recent projects:', error);
    }
}

/**
 * Update metadata for a recent project (track count, duration, etc.)
 */
export function updateRecentProjectMetadata(
    path: string,
    metadata: Partial<Pick<RecentProject, 'trackCount' | 'duration'>>
): void {
    try {
        const projects = getRecentProjects();
        const index = projects.findIndex(p => p.path === path);

        if (index !== -1) {
            projects[index] = {
                ...projects[index],
                ...metadata
            };
            localStorage.setItem(STORAGE_KEY, JSON.stringify(projects));
        }
    } catch (error) {
        console.error('Failed to update project metadata:', error);
    }
}

/**
 * Check if a project exists in recent list
 */
export function isInRecentProjects(path: string): boolean {
    const projects = getRecentProjects();
    return projects.some(p => p.path === path);
}

/**
 * Get a specific recent project by path
 */
export function getRecentProject(path: string): RecentProject | undefined {
    const projects = getRecentProjects();
    return projects.find(p => p.path === path);
}

/**
 * Format relative time for display (e.g., "2h ago", "3d ago")
 */
export function formatRelativeTime(timestamp: number): string {
    const now = Date.now();
    const diffMs = now - timestamp;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    const date = new Date(timestamp);
    return date.toLocaleDateString();
}

/**
 * Get recent projects for display in menu (limited to 5)
 */
export function getRecentProjectsForMenu(): Array<{ path: string; name: string }> {
    return getRecentProjects()
        .slice(0, 5)
        .map(p => ({ path: p.path, name: p.name }));
}
