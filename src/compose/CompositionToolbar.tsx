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
  /**
   * Dans la vue Diapositive (D-3.6-f) : tourner, étirer, déformer, aligner,
   * répartir et redresser agissent dans cette diapositive seulement ; les
   * bulles le disent, comme pour miroir, ordre et groupe qui changent le
   * dessin partout. `adjusted` : un dessin choisi est retouché ici.
   */
  slide?: { adjusted: boolean };
  /** Outil « Masque » (D-4.2-u) : pris ou non, et de quoi le prendre ou le quitter. */
  mask?: { on: boolean; toggle: () => void };
}

export function CompositionToolbar({ selected, actions, enabled, corners = false, slide, mask }: Props) {
  const has = enabled && selected.length > 0;
  // Bulles : ce qui reste dans la diapositive, ce qui change le dessin partout.
  const here = (label: string) => (slide ? `${label}, dans cette diapositive seulement` : label);
  const everywhere = (label: string) => (slide ? `${label} (sur la façade et dans toutes les diapositives)` : label);
  // Unités à aligner : un groupe compte pour un.
  const units = new Set(selected.map((p) => p.group ?? `seul:${p.id}`)).size;
  const grouped = selected.some((p) => p.group);
  const deformed = slide ? slide.adjusted : selected.some((p) => p.deformation);
  return (
    <>
      <ToolGroup>
        <IconButton icon="rotateLeft" label={here("Tourner d'un quart de tour vers la gauche")} onClick={() => actions.rotateBy(-90)} disabled={!has} />
        <IconButton icon="rotateRight" label={here("Tourner d'un quart de tour vers la droite")} onClick={() => actions.rotateBy(90)} disabled={!has} />
        <IconButton icon="flipHorizontal" label={everywhere("Miroir : échanger la gauche et la droite")} onClick={() => actions.flip("horizontal")} disabled={!has} />
        <IconButton icon="flipVertical" label={everywhere("Miroir : échanger le haut et le bas")} onClick={() => actions.flip("vertical")} disabled={!has} />
      </ToolGroup>
      <ToolSeparator />
      <ToolGroup>
        <IconButton
          icon="warpCorners"
          label={corners ? "Revenir au mode normal : cadre et poignées (Échap)" : here("Mode 4 coins : tirer les coins, en perspective")}
          active={has && corners}
          onClick={() => actions.setMode(corners ? "normal" : "corners")}
          disabled={!has}
        />
        <MenuButton
          label={here("Tendre une grille de déformation")}
          icon="warpGrid"
          iconOnly
          variant="quiet"
          disabled={!has}
          items={GRIDS.map(([c, r]) => ({ label: `Grille ${c} × ${r}`, onSelect: () => actions.deform(c, r) }))}
        />
        <IconButton icon="straighten" label={slide ? "Comme sur la façade : retirer la retouche de cette diapositive" : "Redresser : le dessin redevient droit"} onClick={() => actions.deform(0, 0)} disabled={!has || !deformed} />
        {mask && (
          <IconButton
            icon="mask"
            label={
              mask.on
                ? "Quitter l'outil Masque (Échap)"
                : everywhere("Masque : peindre sur le dessin choisi pour en cacher des parties, sans l'effacer")
            }
            shortcut="m"
            active={mask.on}
            onClick={mask.toggle}
            disabled={!enabled || (!has && !mask.on)}
          />
        )}
      </ToolGroup>
      <ToolSeparator />
      <ToolGroup>
        <IconButton icon="alignLeft" label={here("Aligner à gauche")} onClick={() => actions.align("left")} disabled={!has || units < 2} />
        <IconButton icon="alignHCenter" label={here("Centrer horizontalement")} onClick={() => actions.align("hcenter")} disabled={!has || units < 2} />
        <IconButton icon="alignRight" label={here("Aligner à droite")} onClick={() => actions.align("right")} disabled={!has || units < 2} />
        <IconButton icon="alignTop" label={here("Aligner en haut")} onClick={() => actions.align("top")} disabled={!has || units < 2} />
        <IconButton icon="alignVCenter" label={here("Centrer verticalement")} onClick={() => actions.align("vcenter")} disabled={!has || units < 2} />
        <IconButton icon="alignBottom" label={here("Aligner en bas")} onClick={() => actions.align("bottom")} disabled={!has || units < 2} />
        <IconButton icon="distributeHorizontal" label={here("Répartir : même écart horizontal")} onClick={() => actions.distribute("horizontal")} disabled={!has || units < 3} />
        <IconButton icon="distributeVertical" label={here("Répartir : même écart vertical")} onClick={() => actions.distribute("vertical")} disabled={!has || units < 3} />
      </ToolGroup>
      <ToolSeparator />
      <ToolGroup>
        <IconButton icon="bringFront" label={everywhere("Mettre au premier plan")} shortcut="mod+shift+]" onClick={() => actions.arrange("front")} disabled={!has} />
        <IconButton icon="bringForward" label={everywhere("Avancer d'un cran")} shortcut="mod+]" onClick={() => actions.arrange("forward")} disabled={!has} />
        <IconButton icon="sendBackward" label={everywhere("Reculer d'un cran")} shortcut="mod+[" onClick={() => actions.arrange("backward")} disabled={!has} />
        <IconButton icon="sendBack" label={everywhere("Mettre à l'arrière-plan")} shortcut="mod+shift+[" onClick={() => actions.arrange("back")} disabled={!has} />
        <IconButton icon="group" label={everywhere("Grouper")} shortcut="mod+g" onClick={actions.group} disabled={!has || selected.length < 2} />
        <IconButton icon="ungroup" label={everywhere("Dégrouper")} shortcut="mod+shift+g" onClick={actions.ungroup} disabled={!has || !grouped} />
      </ToolGroup>
    </>
  );
}
