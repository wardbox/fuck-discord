import type { AuthResponse, User } from '$lib/protocol/types';

class AuthStore {
	user = $state<User | null>(null);
	sessionId = $state<string | null>(null);
	isAuthenticated = $derived(this.user !== null);

	constructor() {
		// Restore session from localStorage
		const stored = localStorage.getItem('relay_session');
		if (stored) {
			try {
				const data = JSON.parse(stored);
				this.sessionId = data.session_id;
				this.user = data.user;
			} catch {
				localStorage.removeItem('relay_session');
			}
		}
	}

	async register(username: string, password: string, inviteCode: string): Promise<void> {
		const res = await fetch('/api/auth/register', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ username, password, invite_code: inviteCode })
		});

		if (!res.ok) {
			const err = await res.json();
			throw new Error(err.error || 'Registration failed');
		}

		const data: AuthResponse = await res.json();
		this.user = data.user;
		this.sessionId = data.session_id;
		localStorage.setItem('relay_session', JSON.stringify(data));
	}

	async login(username: string, password: string): Promise<void> {
		const res = await fetch('/api/auth/login', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ username, password })
		});

		if (!res.ok) {
			const err = await res.json();
			throw new Error(err.error || 'Login failed');
		}

		const data: AuthResponse = await res.json();
		this.user = data.user;
		this.sessionId = data.session_id;
		localStorage.setItem('relay_session', JSON.stringify(data));
	}

	async logout(): Promise<void> {
		await fetch('/api/auth/logout', { method: 'POST' }).catch(() => {});
		this.user = null;
		this.sessionId = null;
		localStorage.removeItem('relay_session');
	}

	async checkSession(): Promise<boolean> {
		if (!this.sessionId) return false;

		try {
			const res = await fetch('/api/auth/me');
			if (res.ok) {
				const user: User = await res.json();
				this.user = user;
				return true;
			}
		} catch {
			// Session invalid
		}

		this.user = null;
		this.sessionId = null;
		localStorage.removeItem('relay_session');
		return false;
	}
}

export const auth = new AuthStore();
