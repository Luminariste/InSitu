// Calque « Masque » de la vue Diapositive (D-3.6-h), en haut de l'onglet
// Calques : l'œil le cache (ni montré ni gravé), le lasso et la plume
// ajoutent des formes, le choix « dedans / dehors » dit ce qui devient noir
// à l'export.

import { Button, IconButton, List, ListRow, Section, Segmented } from "../ui";
import type { SlideMaskTool } from "./useSlideMask";

export function SlideMaskSection({ tool, blocked }: { tool: SlideMaskTool; blocked: string | null }) {
  if (blocked) return null;
  const mask = tool.mask;
  const shapes = mask?.shapes ?? [];
  const shown = !mask?.hidden;
  return (
    <Section
      title="Masque"
      actions={
        <IconButton
          icon={shown ? "eye" : "eyeOff"}
          label={shown ? "Cacher le masque : il n'est plus montré ni appliqué à l'export" : "Montrer le masque et l'appliquer à l'export"}
          onClick={tool.toggleHidden}
          disabled={!mask}
        />
      }
    >
      <p className="ui-note">
        Tracer une forme sur la photo : à l'export, ce qui est {mask?.outside ? "en dehors" : "dedans"} devient noir, fresque et dessins
        compris.
      </p>
      <div className="slide-mask-tools">
        <Button icon="lasso" active={tool.pen === "lasso"} onClick={() => tool.setPen(tool.pen === "lasso" ? null : "lasso")} title="Entourer une zone en glissant">
          Lasso
        </Button>
        <Button icon="pencil" active={tool.pen === "pen"} onClick={() => tool.setPen(tool.pen === "pen" ? null : "pen")} title="Cliquer les points de la forme, un par un">
          Plume
        </Button>
      </div>
      <Segmented
        label="Ce qui devient noir"
        value={mask?.outside ? "dehors" : "dedans"}
        onChange={(v) => tool.setOutside(v === "dehors")}
        options={[
          { value: "dedans", label: "Noir dedans", title: "L'intérieur des formes devient noir" },
          { value: "dehors", label: "Noir dehors", title: "Tout devient noir sauf l'intérieur des formes" },
        ]}
      />
      {shapes.length > 0 && (
        <List label="Formes du masque">
          {shapes.map((shape, i) => (
            <ListRow
              key={i}
              selected={tool.selected === i}
              onSelect={() => tool.setSelected(tool.selected === i ? null : i)}
              dimmed={!shown}
              title={`Forme ${i + 1}`}
              trailing={`${shape.length} points`}
              actions={<IconButton icon="trash" label="Retirer cette forme" size="sm" onClick={() => tool.remove(i)} />}
            />
          ))}
        </List>
      )}
      {mask && (
        <Button icon="trash" variant="quiet" onClick={() => tool.save(null)}>
          Retirer le masque
        </Button>
      )}
    </Section>
  );
}
