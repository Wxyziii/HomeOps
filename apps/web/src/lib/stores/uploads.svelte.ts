export type UploadStatus = 'preparing' | 'uploading' | 'finalizing' | 'completed' | 'failed';

export type UploadItem = {
	id: string;
	filename: string;
	destinationPath: string;
	relativePath: string;
	percent: number;
	uploadedBytes: number;
	totalBytes: number;
	status: UploadStatus;
	error: string | null;
	createdAt: number;
	updatedAt: number;
};

class UploadStore {
	uploads = $state<UploadItem[]>([]);

	start(files: File[], destinationPath: string) {
		const now = Date.now();
		const normalizedDestination = normalizePath(destinationPath);
		const batchId = `${now}-${Math.random().toString(36).slice(2)}`;
		const items = files.map((file, index) => {
			const relativePath = normalizedDestination ? `${normalizedDestination}/${file.name}` : file.name;
			return {
				id: `${batchId}-${index}`,
				filename: file.name,
				destinationPath: normalizedDestination || 'workspace root',
				relativePath,
				percent: 0,
				uploadedBytes: 0,
				totalBytes: file.size,
				status: 'preparing' as const,
				error: null,
				createdAt: now,
				updatedAt: now
			};
		});
		this.uploads = [...items, ...this.uploads].slice(0, 20);
		return items;
	}

	updateProgress(ids: string[], uploadedBytes: number, totalBytes: number) {
		const now = Date.now();
		const percent = totalBytes > 0 ? Math.min(99, Math.round((uploadedBytes / totalBytes) * 100)) : 0;
		const perItemUploaded = ids.length > 0 ? Math.round(uploadedBytes / ids.length) : uploadedBytes;
		const perItemTotal = ids.length > 0 ? Math.round(totalBytes / ids.length) : totalBytes;
		this.uploads = this.uploads.map((upload) =>
			ids.includes(upload.id)
				? {
						...upload,
						uploadedBytes: Math.min(upload.totalBytes || perItemTotal, perItemUploaded),
						totalBytes: upload.totalBytes || perItemTotal,
						percent,
						status: 'uploading',
						updatedAt: now
					}
				: upload
		);
	}

	setStatus(ids: string[], status: UploadStatus, error: string | null = null) {
		const now = Date.now();
		this.uploads = this.uploads.map((upload) =>
			ids.includes(upload.id)
				? {
						...upload,
						status,
						error,
						percent: status === 'completed' ? 100 : upload.percent,
						updatedAt: now
					}
				: upload
		);
	}

	clearFinished() {
		this.uploads = this.uploads.filter((upload) => upload.status !== 'completed' && upload.status !== 'failed');
	}

	isUploadingPath(path: string) {
		const normalized = normalizePath(path);
		return this.uploads.some(
			(upload) =>
				upload.relativePath === normalized &&
				(upload.status === 'preparing' || upload.status === 'uploading' || upload.status === 'finalizing')
		);
	}

	activeForPath(path: string) {
		const normalized = normalizePath(path);
		return this.uploads.find(
			(upload) =>
				upload.relativePath === normalized &&
				(upload.status === 'preparing' || upload.status === 'uploading' || upload.status === 'finalizing')
		);
	}
}

function normalizePath(value: string) {
	return value.trim().replace(/^\/+|\/+$/g, '');
}

export const uploadState = new UploadStore();
