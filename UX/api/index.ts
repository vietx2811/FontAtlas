// Mọi lệnh backend đi qua một interface; tên phương thức = tên lệnh 6a (camelCase).
import type { LibraryApi } from '../features/library-grid/api';
import type {
  ActivationStateDto, CollectionDto, FaceDetailDto, FaceKeyDto, FamilyStyleDto, SettingsDto,
  SourceDto, TagDto, UiStateDto,
} from './types';

export * from './types';

export interface Api extends LibraryApi {
  // Nguồn
  listSources(): Promise<SourceDto[]>;
  /** Ném CmdError('nested_source', …, { existingId }) khi nằm trong/chứa nguồn có sẵn. */
  addSource(path: string): Promise<SourceDto>;
  removeSource(sourceId: number): Promise<void>;
  rescanSources(sourceIds?: number[]): Promise<void>;
  // Thư viện
  getFamilyStyles(family: string): Promise<FamilyStyleDto[]>;
  setFavorite(face: FaceKeyDto, on: boolean): Promise<void>;
  listCollections(): Promise<CollectionDto[]>;
  createCollection(name: string, color: string | null): Promise<CollectionDto>;
  renameCollection(id: number, name: string): Promise<void>;
  deleteCollection(id: number): Promise<void>;
  addToCollection(id: number, faces: FaceKeyDto[]): Promise<void>;
  removeFromCollection(id: number, faces: FaceKeyDto[]): Promise<void>;
  listTags(): Promise<TagDto[]>;
  tagsOf(face: FaceKeyDto): Promise<TagDto[]>;
  createTag(name: string): Promise<TagDto>;
  tagFaces(tagId: number, faces: FaceKeyDto[]): Promise<void>;
  untagFaces(tagId: number, faces: FaceKeyDto[]): Promise<void>;
  revealInFinder(face: FaceKeyDto): Promise<void>;
  getUiState(): Promise<UiStateDto>;
  setUiState(state: UiStateDto): Promise<void>;
  // Phân tích
  getFace(face: FaceKeyDto, lang: 'vi' | 'en'): Promise<FaceDetailDto>;
  // Cài đặt
  getSettings(): Promise<SettingsDto>;
  // Kích hoạt (hợp đồng Plan 7)
  activationState(): Promise<ActivationStateDto[]>;
  activationActivate(faces: FaceKeyDto[]): Promise<void>;
  activationDeactivate(faces: FaceKeyDto[]): Promise<void>;
}

let current: Api | null = null;
export function setApi(api: Api): void { current = api; }
export function getApi(): Api {
  if (!current) throw new Error('Api chưa được cài: gọi setApi() trước khi render');
  return current;
}
