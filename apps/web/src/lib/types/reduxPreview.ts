export type WeaponPreviewSourceKind =
	| 'clean_gta_export'
	| 'gunpack_export'
	| 'demo_placeholder'
	| 'manual_glb'
	| 'unsupported_gta_resource'
	| 'clean_gta_sample'
	| 'gunpack_sample';

export type WeaponPreviewStatus =
	| 'ready'
	| 'missing_glb'
	| 'converter_not_configured'
	| 'converter_failed'
	| 'unsupported_source_format'
	| 'template_export_ready'
	| 'reverse_export_ready'
	| 'converter_not_connected'
	| 'failed';

export type WeaponPreviewModelFormat = 'glb' | 'gltf';

export type WeaponSourceFileRole =
	| 'drawable_model'
	| 'high_drawable_model'
	| 'fragment_model'
	| 'drawable_dictionary'
	| 'texture_dictionary'
	| 'metadata'
	| 'unknown'
	| 'model'
	| 'high_model'
	| 'fragment';

export type WeaponSourceFileStatus = 'present' | 'missing' | 'unsupported' | 'used';

export type WeaponConversionStatus =
	| 'ready'
	| 'unsupported'
	| 'failed'
	| 'not_configured'
	| 'incomplete';

export type WeaponReverseValidationStatus =
	| 'not_configured'
	| 'unsupported'
	| 'incomplete'
	| 'failed'
	| 'ready'
	| 'validated';

export interface WeaponPreviewModel {
	url: string;
	format: WeaponPreviewModelFormat;
	sizeBytes?: number | null;
	sha256?: string | null;
	hash?: string | null;
	generatedAt?: string | null;
	validationStatus?: string | null;
	meshCount?: number | null;
	primitiveCount?: number | null;
	vertexCount?: number | null;
	indexCount?: number | null;
	materialCount?: number | null;
	boundingBox?: unknown;
	realGtaModel?: boolean;
}

export interface WeaponPreviewSourceFile {
	fileName: string;
	relativePath?: string | null;
	extension: string;
	role: WeaponSourceFileRole;
	sizeBytes?: number | null;
	sha256?: string | null;
	status: WeaponSourceFileStatus;
}

export interface WeaponTextureDictionary {
	fileName: string;
	textureNames?: string[];
	status: string;
}

export interface WeaponMaterialSlot {
	slotName: string;
	textureName?: string | null;
	sourceYtd?: string | null;
	status: string;
	notes?: string | null;
}

export type WeaponTextureSlot = WeaponMaterialSlot;

export interface WeaponConversionInfo {
	gtaToGlbStatus: WeaponConversionStatus;
	glbToGtaStatus: WeaponConversionStatus;
	converterName?: string | null;
	converterVersion?: string | null;
	logsPath?: string | null;
	warnings: string[];
}

export interface WeaponReverseTemplate {
	templateWeaponName?: string | null;
	originalSourceFiles: string[];
	requiredFiles: string[];
	stagedOutputDir?: string | null;
	validationStatus: WeaponReverseValidationStatus;
}

export interface WeaponPreviewManifest {
	manifestVersion: string;
	previewId: string;
	displayName: string;
	weaponName: string;
	weaponPrefix?: string | null;
	sourceLabel: string;
	sourceKind: WeaponPreviewSourceKind;
	previewStatus: WeaponPreviewStatus;
	modelPreview?: WeaponPreviewModel | null;
	previewModel?: WeaponPreviewModel | null;
	sourceFiles: WeaponPreviewSourceFile[];
	textureDictionaries: WeaponTextureDictionary[];
	materialSlots: WeaponMaterialSlot[];
	textureSlots: WeaponTextureSlot[];
	conversion: WeaponConversionInfo;
	reverseTemplate: WeaponReverseTemplate;
	warnings: string[];
	notes: string[];
	buildReady?: false;
}
