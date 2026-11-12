// Visionneuse haute résolution : tout ce que le reste de l'interface en utilise.
export { Viewer, formatFrameStats, type ViewerHandle, type ViewerProps } from "./Viewer";
export { ViewerScene, type FrameStats, type SceneOptions, type SceneStatus } from "./scene";
export type {
  FrameContext,
  LayerBlend,
  LayerSpec,
  MeshImageLayerSpec,
  PolygonShape,
  PolygonsLayerSpec,
  SceneNode,
  TiledLayerSpec,
  TileSource,
} from "./nodes";
export { placementForWidth } from "./geometry";
export type { Placement, Point, Pyramid, PyramidLevel, Rect, Size, ViewState } from "./geometry";
