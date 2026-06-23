export type WeaponPreviewSourceKind =
	| 'clean_gta_sample'
	| 'gunpack_sample'
	| 'demo_placeholder'
	| 'manual_glb'
	| 'unsupported_gta_resource';

export type WeaponPreviewStatus =
	| 'ready'
	| 'missing_glb'
	| 'converter_not_connected'
	| 'unsupported_source_format'
	| 'failed';

export type WeaponPreviewModelFormat = 'glb' | 'gltf';

export type WeaponSourceFileRole =
	| 'model'
	| 'high_model'
	| 'fragment'
	| 'drawable_dictionary'
	| 'texture_dictionary'
	| 'metadata';

export type WeaponSourceFileStatus = 'present' | 'missing' | 'unsupported';

export interface WeaponPreviewModel {
	url: string;
	format: WeaponPreviewModelFormat;
	sizeBytes?: number | null;
	hash?: string | null;
}

export interface WeaponPreviewSourceFile {
	fileName: string;
	extension: string;
	role: WeaponSourceFileRole;
	status: WeaponSourceFileStatus;
}

export interface WeaponTextureSlot {
	slotName: string;
	textureName: string;
	status: string;
	notes?: string | null;
}

export interface WeaponPreviewManifest {
	manifestVersion: string;
	previewId: string;
	displayName: string;
	weaponName: string;
	sourceLabel: string;
	sourceKind: WeaponPreviewSourceKind;
	previewStatus: WeaponPreviewStatus;
	previewModel?: WeaponPreviewModel | null;
	sourceFiles: WeaponPreviewSourceFile[];
	textureSlots: WeaponTextureSlot[];
	warnings: string[];
	notes: string[];
}
