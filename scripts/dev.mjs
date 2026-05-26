import net from 'node:net';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const webDir = path.join(repoRoot, 'apps', 'web');
const isWindows = process.platform === 'win32';
const children = new Map();

const cliMode = readCliMode();
const mode = cliMode ?? process.env.HOMEOPS_DEV_MODE ?? 'tunnel';
const sshHost = process.env.HOMEOPS_SSH_HOST ?? 'homeops';
const tunnelLocalPort = Number(process.env.HOMEOPS_TUNNEL_LOCAL_PORT ?? '8787');
const tunnelRemoteHost = process.env.HOMEOPS_TUNNEL_REMOTE_HOST ?? '127.0.0.1';
const tunnelRemotePort = Number(process.env.HOMEOPS_TUNNEL_REMOTE_PORT ?? '8787');

if (!['local', 'tunnel'].includes(mode)) {
	log(`invalid HOMEOPS_DEV_MODE "${mode}". Use "local" or "tunnel".`);
	process.exit(1);
}

process.on('SIGINT', () => shutdown(130));
process.on('SIGTERM', () => shutdown(143));
process.on('exit', () => {
	for (const child of children.values()) {
		killChild(child);
	}
});

await main();

async function main() {
	log(`mode=${mode}`);

	if (mode === 'tunnel') {
		await startTunnelIfNeeded();
	} else {
		await startLocalBackendIfNeeded();
	}

	startTauriDev();
}

async function startTunnelIfNeeded() {
	const portInUse = await isPortInUse(tunnelLocalPort, '127.0.0.1');
	if (portInUse) {
		log(`tunnel already active or port ${tunnelLocalPort} is already in use`);
		return;
	}

	log('starting tunnel');
	spawnManaged(
		'tunnel',
		'ssh',
		[
			'-N',
			'-o',
			'ExitOnForwardFailure=yes',
			'-L',
			`${tunnelLocalPort}:${tunnelRemoteHost}:${tunnelRemotePort}`,
			sshHost
		],
		repoRoot
	);
}

async function startLocalBackendIfNeeded() {
	const portInUse = await isPortInUse(8787, '127.0.0.1');
	if (portInUse) {
		log('local backend already active or port 8787 is already in use');
		return;
	}

	log('starting local backend');
	spawnManaged('backend', cargoCommand(), ['run', '-p', 'server-agent'], repoRoot);
}

function startTauriDev() {
	log('starting Tauri dev');
	const npm = npmInvocation(['run', 'tauri:dev']);
	spawnManaged('tauri', npm.command, npm.args, webDir, true);
}

function spawnManaged(name, command, args, cwd, exitWithProcess = false) {
	const child = spawn(command, args, {
		cwd,
		stdio: ['inherit', 'pipe', 'pipe'],
		env: process.env,
		shell: false
	});
	children.set(name, child);
	child.stdout?.on('data', (chunk) => process.stdout.write(chunk));
	child.stderr?.on('data', (chunk) => process.stderr.write(chunk));

	child.on('exit', (code, signal) => {
		children.delete(name);
		if (signal) {
			log(`${name} stopped by ${signal}`);
		} else {
			log(`${name} exited with code ${code ?? 0}`);
		}

		if (exitWithProcess) {
			shutdown(code ?? 0);
		}
	});

	child.on('error', (error) => {
		children.delete(name);
		log(`${name} failed to start: ${error.message}`);
		if (exitWithProcess) {
			shutdown(1);
		}
	});

	return child;
}

function readCliMode() {
	for (const arg of process.argv.slice(2)) {
		if (arg.startsWith('--mode=')) {
			return arg.slice('--mode='.length);
		}
	}
	return null;
}

function isPortInUse(port, host) {
	return new Promise((resolve) => {
		const socket = net.createConnection({ port, host });
		socket.once('connect', () => {
			socket.destroy();
			resolve(true);
		});
		socket.once('error', () => {
			socket.destroy();
			resolve(false);
		});
		socket.setTimeout(1200, () => {
			socket.destroy();
			resolve(false);
		});
	});
}

function cargoCommand() {
	return isWindows ? 'cargo.exe' : 'cargo';
}

function npmInvocation(args) {
	if (!isWindows) {
		return { command: 'npm', args };
	}
	return {
		command: process.env.ComSpec ?? 'cmd.exe',
		args: ['/d', '/s', '/c', 'npm.cmd', ...args]
	};
}

function shutdown(code = 0) {
	log('shutdown');
	for (const child of children.values()) {
		killChild(child);
	}
	children.clear();
	process.exit(code);
}

function killChild(child) {
	if (!child || child.killed) return;
	if (isWindows && child.pid) {
		spawn('taskkill', ['/pid', String(child.pid), '/t', '/f'], {
			stdio: 'ignore',
			windowsHide: true
		});
		return;
	}
	child.kill('SIGTERM');
}

function log(message) {
	console.log(`[homeops-dev] ${message}`);
}
