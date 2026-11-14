// Réglages de l'outil « Masque » (D-4.2-u), dans la barre du haut quand
// l'outil est pris : pinceau ou lasso, épaisseur du pinceau, tout
// réafficher. Le même en Façade et en Diapositive.

import { Button, MenuButton, Segmented, ToolGroup, ToolSeparator } from "../ui";
import { BRUSH_SIZES_PX } from "./mask";
import type { MaskTool } from "./useMaskTool";

export function MaskBar({ tool, count }: { tool: MaskTool; count: number }) {
  return (
    <>
      <ToolGroup label="Masque">
        <Segmented
          label="Forme du masque"
          value={tool.shape}
          onChange={tool.setShape}
          options={[
            { value: "brush", label: "Pinceau", shortcut: "t", title: "Peindre pour cacher ; ⌥ + peindre pour faire réapparaître" },
            { value: "lasso", label: "Lasso", shortcut: "z", title: "Entourer une zone pour la cacher d'un coup ; ⌥ pour la faire réapparaître" },
          ]}
        />
        <MenuButton
          label={`Épaisseur : ${tool.brushPx} pt`}
          variant="quiet"
          disabled={tool.shape !== "brush"}
          title="Épaisseur du pinceau, en points d'écran ([ plus fin, ] plus épais)"
          items={BRUSH_SIZES_PX.map((px) => ({ label: `${px} pt`, checked: px === tool.brushPx, onSelect: () => tool.setBrushPx(px) }))}
        />
      </ToolGroup>
      <ToolSeparator />
      <span className="compose-mask-count">
        {count === 0 ? "Choisir un dessin à masquer" : count === 1 ? "1 dessin choisi" : `${count} dessins choisis`}
      </span>
      <Button icon="eye" onClick={tool.clear} disabled={!tool.masked} title="Retirer le masque des dessins choisis : tout réapparaît">
        Tout réafficher
      </Button>
      <Button onClick={() => tool.setOn(false)} shortcut="escape">
        Terminer
      </Button>
    </>
  );
}
