export function formatTimestamp(isoString: string): string {
	const date = new Date(isoString + 'Z'); // Server sends UTC without Z
	const now = new Date();
	const isToday = date.toDateString() === now.toDateString();

	const yesterday = new Date(now);
	yesterday.setDate(yesterday.getDate() - 1);
	const isYesterday = date.toDateString() === yesterday.toDateString();

	const time = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

	if (isToday) return time;
	if (isYesterday) return `Yesterday ${time}`;
	return `${date.toLocaleDateString([], { month: 'short', day: 'numeric' })} ${time}`;
}

export function formatCompactTimestamp(isoString: string): string {
	const date = new Date(isoString + 'Z');
	return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
}
