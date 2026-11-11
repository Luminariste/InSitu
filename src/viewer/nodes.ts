// Ce que la visionneuse sait afficher.
//
// La scène est un arbre PixiJS (WebGL). Son conteneur `world` est gradué en
// millimètres : tout objet qu'on y pose se place et se dimensionne en mm, et
// suit le zoom et le déplacement sans rien faire.
//
// POINT D'ANCRAGE pour les objets à venir (images non tuilées, dessins
// déformés par maillage, tracés, emprises) : écrire une classe qui respecte
// `SceneNode`, ajouter sa description à `LayerSpec`, et la créer dans
// `createNode` (scene.ts). Rien d'autre ne change. Un objet ponctuel peut
// aussi être ajouté directement par `ViewerScene.addNode`.

import type { Container, Texture } from "pixi.js";
import type { Placement, Pyramid, Rect, Size, ViewState } from "./geometry";
import type { TileStore } from "./tileStore";

/** Une pyramide de tuiles et l'adresse où la lire. */
export interface TileSource {
  pyramid: Pyramid;
  /** Adresse du dossier de la pyramide, sans barre finale. */
  baseUrl: string;
}

export type LayerBlend = "normal" | "screen";

interface LayerBase {
  /** Identifiant stable : il permet de mettre à jour un calque sans le recréer. */
  id: string;
  /** De 0 (invisible) à 1 (opaque). 1 par défaut. */
  opacity?: number;
  /** `true` par défaut. */
  visible?: boolean;
  /**
   * `screen` éclaircit ce qui est dessous au lieu de le recouvrir : le noir
   * d'un dessin devient transparent, ses traits blancs restent.
   */
  blend?: LayerBlend;
}

/** Grande image affichée par tuiles. */
export interface TiledLayerSpec extends LayerBase {
  kind: "tiles";
  source: TileSource;
  placement: Placement;
}

/** Tous les types de calques. Les prochains s'ajoutent à cette union. */
export type LayerSpec = TiledLayerSpec;

/** Ce qu'un objet de la scène reçoit avant chaque image. */
export interface FrameContext {
  view: ViewState;
  /** Taille de la visionneuse, en pixels CSS. */
  viewport: Size;
  devicePixelRatio: number;
  tiles: TileStore<Texture>;
  /** Demande une tuile pour cette image (plus `priority` est petit, plus c'est urgent). */
  wantTile(url: string, priority: number): void;
}

/** Un objet de la scène. */
export interface SceneNode {
  /** Objet PixiJS posé dans le repère commun (unités : mm). */
  readonly container: Container;
  /** Place occupée dans le repère commun, en mm ; `null` si rien à montrer. */
  bounds(): Rect | null;
  /** Côté du plus petit détail, en mm (sert au zoom 100 %) ; `null` si sans objet. */
  finestDetail(): number | null;
  /** Appelée avant chaque image : l'objet se met à jour pour la vue donnée. */
  update(frame: FrameContext): void;
  destroy(): void;
}
