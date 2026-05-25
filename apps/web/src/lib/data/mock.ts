export type StatusKind = 'running' | 'done' | 'failed' | 'queued' | 'ok' | 'warn' | 'error' | 'offline';

export const metrics = [
	{ label: 'CPU', icon: 'ti-cpu', value: '23', unit: '%', sub: '4 cores · 3.2 GHz', percent: 23, color: '#8db9e4' },
	{ label: 'Memory', icon: 'ti-device-desktop-analytics', value: '6.1', unit: ' GB', sub: 'of 8 GB · 76%', percent: 76, color: '#b8b2a3' },
	{ label: 'Storage', icon: 'ti-database', value: '1.8', unit: ' TB', sub: 'of 4 TB · 45%', percent: 45, color: '#2f8f1f' },
	{ label: 'Network', icon: 'ti-wifi', value: '42', unit: ' MB/s', sub: '↑ 12 ↓ 30 MB/s', percent: 42, color: '#b9852d' }
];

export const jobs = [
	{ name: 'nightly-backup', time: '2m ago', status: 'running' as const, icon: 'ti-refresh' },
	{ name: 'media-transcode', time: '18m ago', status: 'done' as const, icon: 'ti-check' },
	{ name: 'db-snapshot', time: '1h ago', status: 'failed' as const, icon: 'ti-alert-triangle' },
	{ name: 'log-rotate', time: 'in 3h', status: 'queued' as const, icon: 'ti-clock' }
];

export const logs = [
	{ time: '09:41:02', level: 'ERR', kind: 'error' as const, message: 'db-snapshot: connection refused on :5432' },
	{ time: '09:23:17', level: 'OK', kind: 'ok' as const, message: 'media-transcode: 4 files processed' },
	{ time: '09:20:05', level: 'INF', kind: 'info' as const, message: 'nightly-backup: started /mnt/data' },
	{ time: '08:55:44', level: 'WRN', kind: 'warn' as const, message: 'memory usage above 75% threshold' },
	{ time: '08:30:00', level: 'INF', kind: 'info' as const, message: 'system: uptime 14d 6h 12m' }
];

export const disks = [
	{ mount: '/', percent: 31, color: '#8db9e4' },
	{ mount: '/data', percent: 65, color: '#b8b2a3' },
	{ mount: '/media', percent: 82, color: '#b9852d' },
	{ mount: '/bak', percent: 19, color: '#2f8f1f' }
];

export const files = [
	{ type: 'folder', name: 'Photos', size: '—', modified: '2026-05-22 14:30', permissions: 'drwxr-xr-x' },
	{ type: 'folder', name: 'Videos', size: '—', modified: '2026-05-21 09:10', permissions: 'drwxr-xr-x' },
	{ type: 'image', name: 'holiday_2025.jpg', size: '4.2 MB', modified: '2026-05-20 18:42', permissions: '-rw-r--r--' },
	{ type: 'video', name: 'birthday_clip.mp4', size: '1.1 GB', modified: '2026-05-18 21:05', permissions: '-rw-r--r--' },
	{ type: 'doc', name: 'tax_2025.pdf', size: '820 KB', modified: '2026-04-30 10:00', permissions: '-rw-------' },
	{ type: 'zip', name: 'archive_apr.tar.gz', size: '3.7 GB', modified: '2026-05-01 02:00', permissions: '-rw-r--r--' },
	{ type: 'code', name: 'docker-compose.yml', size: '3.1 KB', modified: '2026-03-14 11:20', permissions: '-rw-r--r--' },
	{ type: 'doc', name: 'notes.md', size: '14 KB', modified: '2026-05-24 08:55', permissions: '-rw-r--r--' }
];

export const modules = [
	{ name: 'AI Redux Maker', href: '/apps/ai-redux-maker', icon: 'ti-brain', status: 'ready', description: 'Build profiles and reports from imported redux knowledge.' },
	{ name: 'Minecraft Manager', href: '/apps', icon: 'ti-cube', status: 'later', description: 'Server lifecycle and world backups planned.' },
	{ name: 'Backup Manager', href: '/apps', icon: 'ti-archive', status: 'later', description: 'Scheduled workspace-safe backup flows planned.' },
	{ name: 'Website Manager', href: '/apps', icon: 'ti-world', status: 'later', description: 'Website and help desk operations planned.' }
];

export const archives = [
	{ name: 'nightly-backup_2026-05-25.tar.gz', meta: 'Full backup · /mnt/data · nightly-backup job', icon: 'ti-package', tone: 'archive', size: '18.4 GB', date: 'Today 02:00', status: 'verified' },
	{ name: 'nightly-backup_2026-05-24.tar.gz', meta: 'Full backup · /mnt/data · nightly-backup job', icon: 'ti-package', tone: 'archive', size: '18.1 GB', date: 'Yesterday 02:00', status: 'verified' },
	{ name: 'db-snapshot_2026-05-24.sql.gz', meta: 'DB snapshot · /var/lib/postgres · db-snapshot job', icon: 'ti-database', tone: 'db', size: '2.3 GB', date: 'Yesterday 03:00', status: 'failed' },
	{ name: 'media-inc_2026-05-23.tar', meta: 'Incremental · /mnt/media · media-transcode job', icon: 'ti-photo', tone: 'media', size: '54.7 GB', date: 'May 23 · 01:00', status: 'verified' },
	{ name: 'nightly-backup_2026-05-23.tar.gz', meta: 'Full backup · /mnt/data · nightly-backup job', icon: 'ti-package', tone: 'archive', size: '17.9 GB', date: 'May 23 · 02:00', status: 'expiring' },
	{ name: 'db-snapshot_2026-05-22.sql.gz', meta: 'DB snapshot · /var/lib/postgres · db-snapshot job', icon: 'ti-database', tone: 'db', size: '2.2 GB', date: 'May 22 · 03:00', status: 'verified' }
];

export const scheduledJobs = [
	{ name: 'nightly-backup', command: 'rsync -avz /mnt/data /mnt/backup', icon: 'ti-refresh', tone: 'info', status: 'running', progress: 62, schedule: '0 2 * * *', lastRun: 'Today 02:00', duration: '~18 min avg', nextRun: 'Tomorrow 02:00', note: 'Started 2m ago' },
	{ name: 'media-transcode', command: 'ffmpeg -i input -c:v h264 output', icon: 'ti-video', tone: 'ok', status: 'done', progress: 100, schedule: '0 * * * *', lastRun: 'Today 09:00', duration: '4 min 12 sec', nextRun: 'Today 10:00', note: 'Completed 18m ago' },
	{ name: 'db-snapshot', command: 'pg_dump -Fc homedb > snapshot.dump', icon: 'ti-database', tone: 'err', status: 'failed', progress: 0, schedule: '0 3 * * *', lastRun: 'Today 03:00', duration: '0 sec (error)', nextRun: 'Tomorrow 03:00', note: 'Error: connection refused :5432' },
	{ name: 'log-rotate', command: 'logrotate /etc/logrotate.conf', icon: 'ti-rotate', tone: 'warn', status: 'queued', progress: 0, schedule: '0 12 * * *', lastRun: 'Yesterday 12:00', duration: '~8 sec avg', nextRun: 'Today 12:00', note: 'Runs in ~3h' }
];

export const logLines = [
	{ line: 1, time: '09:41:02.331', source: 'db-snapshot', level: 'ERR', kind: 'err', message: 'connection refused on 127.0.0.1:5432 - is postgres running?' },
	{ line: 2, time: '09:41:02.330', source: 'db-snapshot', level: 'ERR', kind: 'err', message: 'pg_dump exited with code 1' },
	{ line: 3, time: '09:41:01.887', source: 'db-snapshot', level: 'DBG', kind: 'dbg', message: 'attempting connect to postgres dsn=host=127.0.0.1 port=5432 dbname=homedb' },
	{ line: 4, time: '09:41:01.001', source: 'db-snapshot', level: 'INF', kind: 'info', message: 'job started' },
	{ line: 5, time: '09:23:17.554', source: 'media-transcode', level: 'OK', kind: 'ok', message: '4 files processed - output written to /mnt/media/out' },
	{ line: 6, time: '09:19:04.221', source: 'media-transcode', level: 'INF', kind: 'info', message: 'transcoding holiday_2025.mov -> h264 1080p' },
	{ line: 7, time: '09:19:02.005', source: 'media-transcode', level: 'INF', kind: 'info', message: 'job started - 4 files in queue' },
	{ line: 8, time: '08:55:44.001', source: 'system', level: 'WRN', kind: 'warn', message: 'memory usage 76% - threshold is 75%' },
	{ line: 9, time: '08:20:33.771', source: 'nightly-backup', level: 'OK', kind: 'ok', message: 'backup complete - 18.4 GB written to /mnt/backup' },
	{ line: 10, time: '08:05:14.220', source: 'nightly-backup', level: 'INF', kind: 'info', message: 'syncing /mnt/data -> /mnt/backup (18.1 GB)' },
	{ line: 11, time: '08:00:01.003', source: 'nightly-backup', level: 'INF', kind: 'info', message: 'job started' },
	{ line: 12, time: '07:30:00.000', source: 'system', level: 'INF', kind: 'info', message: 'uptime 14d 6h 12m - load avg 0.42 0.38 0.31' }
];
