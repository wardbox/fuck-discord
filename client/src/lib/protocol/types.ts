// === Data types (match server models) ===

export interface User {
	id: string;
	username: string;
	display_name: string | null;
	avatar_url: string | null;
	status: 'online' | 'idle' | 'dnd' | 'offline';
	created_at: string;
}

export interface Channel {
	id: string;
	name: string;
	topic: string | null;
	category: string | null;
	channel_type: string;
	position: number;
	created_at: string;
}

export interface Reaction {
	emoji: string;
	count: number;
	users: string[];
}

export interface Message {
	id: string;
	channel_id: string;
	author_id: string;
	author_username: string;
	content: string;
	edited_at: string | null;
	created_at: string;
	reactions?: Reaction[];
}

// === Client → Server messages ===

export type ClientMessage =
	| { type: 'authenticate'; token: string }
	| { type: 'send_message'; channel_id: string; content: string; nonce?: string }
	| { type: 'edit_message'; message_id: string; content: string }
	| { type: 'delete_message'; message_id: string }
	| { type: 'add_reaction'; message_id: string; emoji: string }
	| { type: 'remove_reaction'; message_id: string; emoji: string }
	| { type: 'typing'; channel_id: string }
	| { type: 'set_status'; status: 'online' | 'idle' | 'dnd' }
	| { type: 'subscribe'; channel_ids: string[] }
	| { type: 'unsubscribe'; channel_ids: string[] };

// === Server → Client messages ===

export type ServerMessage =
	| { type: 'ready'; user: User; channels: Channel[]; members: User[] }
	| { type: 'message_create'; message: Message; nonce?: string }
	| { type: 'message_update'; message: Message }
	| { type: 'message_delete'; channel_id: string; message_id: string }
	| { type: 'typing_start'; channel_id: string; user_id: string; username: string }
	| { type: 'presence_update'; user_id: string; status: 'online' | 'idle' | 'dnd' | 'offline' }
	| { type: 'channel_create'; channel: Channel }
	| { type: 'channel_update'; channel: Channel }
	| { type: 'channel_delete'; channel_id: string }
	| { type: 'member_join'; user: User }
	| { type: 'member_leave'; user_id: string }
	| { type: 'reaction_update'; channel_id: string; message_id: string; reactions: Reaction[] }
	| { type: 'error'; code: string; message: string };

// === Auth types ===

export interface AuthResponse {
	user: User;
	session_id: string;
}
