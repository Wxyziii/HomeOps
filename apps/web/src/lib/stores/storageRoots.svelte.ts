import { browser } from '$app/environment';

const STORAGE_ROOT_KEY = 'homeops.storageRootId';

class StorageRootStore {
	selectedRootId = $state('main');

	load() {
		if (!browser) return;
		const saved = localStorage.getItem(STORAGE_ROOT_KEY)?.trim();
		if (saved) this.selectedRootId = saved;
	}

	select(rootId: string) {
		const id = rootId.trim() || 'main';
		this.selectedRootId = id;
		if (browser) localStorage.setItem(STORAGE_ROOT_KEY, id);
	}
}

export const storageRoots = new StorageRootStore();
export { STORAGE_ROOT_KEY };
