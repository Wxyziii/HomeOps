import { browser } from '$app/environment';

const STORAGE_ROOT_KEY = 'homeops.storageRootId';
export const ALL_STORAGE_ROOT_ID = 'all';

class StorageRootStore {
	selectedRootId = $state(ALL_STORAGE_ROOT_ID);

	load() {
		if (!browser) return;
		const saved = localStorage.getItem(STORAGE_ROOT_KEY)?.trim();
		if (saved) this.selectedRootId = saved;
	}

	select(rootId: string) {
		const id = rootId.trim() || ALL_STORAGE_ROOT_ID;
		this.selectedRootId = id;
		if (browser) localStorage.setItem(STORAGE_ROOT_KEY, id);
	}
}

export const storageRoots = new StorageRootStore();
export { STORAGE_ROOT_KEY };
