/**
 * Formats a timestamp into a relative time string (e.g., "Just now", "5m ago", "2h ago")
 * @param timestamp Time in milliseconds
 */
export const formatRelativeTime = (timestamp: number): string => {
    if (!timestamp) return 'Never';

    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();

    // Safety check for future dates or clock desync
    if (diffMs < 0) return 'Just now';

    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins} min ago`;
    if (diffHours < 24) return `${diffHours} hr ago`;
    if (diffDays < 7) return `${diffDays} day ago`;

    return date.toLocaleDateString();
};

/**
 * Formats seconds since epoch into relative time
 * @param seconds Seconds since UNIX epoch
 */
export const formatRelativeTimeFromSeconds = (seconds: number): string => {
    return formatRelativeTime(seconds * 1000);
};
