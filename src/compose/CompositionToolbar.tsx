// Outils des dessins posés, en icônes dans la barre d'outils de l'écran
// Façade (demande de Benjamin, D15-j) : tourner, symétries, aligner,
// répartir, déformer, ordre, groupe. Chaque bouton agit sur la sélection et
// se grise quand il ne s'applique pas ; les mesures (centre, largeur,
// rotation) restent dans l'onglet Dessins de l'inspecteur.

import { IconButton, MenuButton, ToolGroup, ToolSeparator } from "../ui";
import type { Placement } from "./placement";
import type { CompositionActions } from "./useComposition";

/** Grilles proposées (colonnes × rangées) : une rangée de six pour une voussure, des grilles carrées. */
export const GRIDS: [number, number][] = [
  [2, 2],
  [3, 3],
  [4, 4],
  [6, 1],
  [6, 2],
  [8, 4],
];

interface Props {
  selected: Placement[];
  actions: CompositionActions;
  /** Faux quand la composition est indisponible (pas de plan, pas d'échelle, geste en cours). */
  enabled: boolean;
  /** Mode 4 coins actif sur la sélection (D-4.2-s). */
  corners?: boolean;
}

export function CompositionToolbar({ selected, actions, enabled, corners = false }: Props) {
  const has = enabled && selected.length > 0;
  // Unités à aligner : un groupe compte pour un.
  const units = new Set(selected.map((p) => p.group ?? `seul:${p.id}`)).size;
  const grouped = selected.some((p) => p.group);
  const deformed = selected.some((p) => p.deformation);
  return (
    <>
      <ToolGroup>
        <IconButton icon="rotateLeft" label="Tourner d'un quart de tour vers la gauche" onClick={() => actions.rotateBy(-90)} disabled={!has} />
        <IconButton icon="rotateRight" label="Tourner d'un quart de tour vers la droite" onClick={() => actions.rotateBy(90)} disabled={!has} />
        <IconButton icon="flipHorizontal" label="Miroir : échanger la gauche et la droite" onClick={() => actions.flip("horizontal")} disabled={!has} />
        <IconButton icon="flipVertical" label="Miroir : échanger le haut et le bas" onClick={() => actions.flip("vertical")} disabled={!has} />
      </ToolGroup>
      <ToolSeparator />
      <ToolGroup>
        <IconButton
          icon="warpCorners"
          label={corners ? "Revenir au mode normal : cadre et poignées (Échap)" : "Mode 4 coins : tirer les coins, en perspective"}
          active={has && corners}
          onClick={() => actions.setMode(corners ? "normal" : "corners")}
          disabled={!has}
        />
        <MenuButton
          label="Tendre une grille de déformation"
          icon="warpGrid"
          iconOnly
          variant="quiet"
          disabled={!has}
          items={GRIDS.map(([c, r]) => ({ label: `Grille ${c} × ${r}`, onSelect: () => actions.deform(c, r) }))}
        />
        <IconButton icon="straighten" label="Redresser : le dessin redevient droit" onClick={() => actions.deform(0, 0)} disabled={!has || !deformed} />
      </ToolGroup>
      <ToolSeparator />
      <ToolGroup>
        <IconButton icon="alignLeft" label="Aligner à gauche" onClick={() => actions.align("left")} disabled={!has || units < 2} />
        <IconButton icon="alignHCenter" label="Centrer horizontalement" onClick={() => actions.align("hcenter")} disabled={!has || units < 2} />
        <IconButton icon="alignRight" label="Aligner à droite" onClick={() => actions.align("right")} disabled={!has || units < 2} />
        <IconButton icon="alignTop" label="Aligner en haut" onClick={() => actions.align("top")} disabled={!has || units < 2} />
        <IconButton icon="alignVCenter" label="Centrer verticalement" onClick={() => actions.align("vcenter")} disabled={!has || units < 2} />
        <IconButton icon="alignBottom" label="Aligner en bas" onClick={() => actions.align("bottom")} disabled={!has || units < 2} />
        <IconButton icon="distributeHorizontal" label="Répartir : même écart horizontal" onClick={() => actions.distribute("horizontal")} disabled={!has || units < 3} />
        <IconButton icon="distributeVertical" label="Répartir : même écart vertical" onClick={() => actions.distribute("vertical")} disabled={!has || units < 3} />
      </ToolGroup>
      <ToolSeparator />
      <ToolGroup>
        <IconButton icon="bringFront" label="Mettre au premier plan" shortcut="mod+shift+]" onClick={() => actions.arrange("front")} disabled={!has} />
        <IconButton icon="bringForward" label="Avancer d'un cran" shortcut="mod+]" onClick={() => actions.arrange("forward")} disabled={!has} />
        <IconButton icon="sendBackward" label="Reculer d'un cran" shortcut="mod+[" onClick={() => actions.arrange("backward")} disabled={!has} />
        <IconButton icon="sendBack" label="Mettre à l'arrière-plan" shortcut="mod+shift+[" onClick={() => actions.arrange("back")} disabled={!has} />
        <IconButton icon="group" label="Grouper" shortcut="mod+g" onClick={actions.group} disabled={!has || selected.length < 2} />
        <IconButton icon="ungroup" label="Dégrouper" shortcut="mod+shift+g" onClick={actions.ungroup} disabled={!has || !grouped} />
      </ToolGroup>
    </>
  );
}
