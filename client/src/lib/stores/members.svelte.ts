import type { User } from '$lib/protocol/types';

class MemberStore {
	members = $state<User[]>([]);

	onlineMembers = $derived(this.members.filter((m) => m.status !== 'offline'));
	offlineMembers = $derived(this.members.filter((m) => m.status === 'offline'));

	setMembers(members: User[]) {
		this.members = members;
	}

	updatePresence(userId: string, status: string) {
		this.members = this.members.map((m) =>
			m.id === userId ? { ...m, status: status as User['status'] } : m
		);
	}

	addMember(user: User) {
		if (!this.members.find((m) => m.id === user.id)) {
			this.members = [...this.members, user];
		}
	}

	removeMember(userId: string) {
		this.members = this.members.filter((m) => m.id !== userId);
	}
}

export const memberStore = new MemberStore();
