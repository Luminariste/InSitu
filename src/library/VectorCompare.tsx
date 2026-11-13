// Comparaison image / vecteurs d'un dessin (tâche 5.2), en plein écran sur
// la bibliothèque. Le dessin et ses courbes sont superposés dans le même
// cadre (pixels du dessin) ; on zoome à la molette ou au pavé tactile, on
// déplace en glissant. À droite, les chiffres de la vectorisation et les
// réglages du dessin : « Appliquer » fait refaire les vecteurs par le moteur
// et enregistre les réglages dans le projet.

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type PointerEvent, type WheelEvent } from "react";
import { DEFAULT_VECTOR_SETTINGS, type VectorSettings, type VectorSummary } from "../engine";
import { classifyWheel, wheelZoomFactor } from "../viewer/geometry";
import {
  deviationVerdict,
  draftOf,
  fitView,
  frNumber,
  isDefault,
  parseDraft,
  sameSettings,
  scaleLabel,
  sliderText,
  MAX_SMOOTHING_PX,
  zoomAt,
  type SettingsDraft,
  type View,
} from "./vectorLogic";
import { BACKDROPS, type Backdrop } from "./backdrop";
import { Button, IconButton, Panel, Property, Section, Segmented, ToolGroup, ToolSeparator, ToolSpacer, Toolbar } from "../ui";

/** Les vecteurs, avec les adresses déjà prêtes à afficher. */
export interface ShownVectors extends VectorSummary {
  imageUrl: string;
  filledUrl: string;
  outlineUrl: string;
}

type Mode = "image" | "vecteurs" | "contour" | "rideau";

const MODES: { value: Mode; label: string; title: string }[] = [
  { value: "image", label: "Image", title: "Le dessin tel qu'il est dans son fichier" },
  { value: "vecteurs", label: "Vecteurs", title: "Les courbes remplies de blanc" },
  { value: "contour", label: "Contour sur l'image", title: "Les courbes en rouge, posées sur l'image" },
  { value: "rideau", label: "Rideau", title: "Image à gauche, vecteurs à droite : glisser le rideau" },
];

/** Réglages, dans l'ordre où on les touche ; un curseur à côté des plus utiles (D-5.2-g). */
const FIELDS: {
  key: keyof VectorSettings;
  label: string;
  unit: string;
  help: string;
  range?: { min: number; max: number; step: number; fallback: number };
}[] = [
  {
    key: "threshold",
    label: "Seuil",
    unit: "1 à 255",
    help: "Un pixel au moins aussi clair fait partie du trait. Plus bas : les gris pâles sont gardés.",
    range: { min: 1, max: 255, step: 1, fallback: 128 },
  },
  {
    key: "smoothingPx",
    label: "Lissage",
    unit: "px",
    help: "Adoucit le bord du trait avant le tracé : efface le tremblement et les petites bosses. 0 : le trait tel quel.",
    range: { min: 0, max: MAX_SMOOTHING_PX, step: 0.5, fallback: 0 },
  },
  {
    key: "tolerancePx",
    label: "Réduction des points",
    unit: "px",
    help: "Écart permis entre les courbes et l'image. Plus grand : moins de points, tracé plus simple, mais moins fidèle.",
    range: { min: 0.05, max: 5, step: 0.05, fallback: 0.3 },
  },
  { key: "cornerAngleDeg", label: "Angle des coins", unit: "°", help: "Au-delà de ce virage, un coin reste vif ; en deçà, la courbe est arrondie." },
  { key: "minAreaPx", label: "Aire minimale", unit: "px²", help: "Les poussières et petits trous plus petits sont retirés. 0 : tout est gardé." },
];

/** Valeur du curseur pour un réglage saisi (« 0,3 ») ; la valeur par défaut si illisible. */
function rangeValue(text: string, fallback: number): number {
  const v = Number(text.trim().replace(/\s/g, "").replace(",", "."));
  return text.trim() !== "" && Number.isFinite(v) ? v : fallback;
}

interface Props {
  name: string;
  vectors: ShownVectors | null;
  backdrop: Backdrop;
  /** Change le fond (le même que celui de la bibliothèque). */
  onBackdrop: (backdrop: Backdrop) => void;
  /** Faux dans le navigateur seul : les réglages ne s'appliquent pas. */
  editable: boolean;
  busy: boolean;
  error: string | null;
  onApply: (settings: VectorSettings | null) => void;
  onClose: () => void;
}

export function VectorCompare({ name, vectors, backdrop, onBackdrop, editable, busy, error, onApply, onClose }: Props) {
  const [mode, setMode] = useState<Mode>("contour");
  const [view, setView] = useState<View | null>(null);
  const [curtain, setCurtain] = useState(0.5);
  const [draft, setDraft] = useState<SettingsDraft>(draftOf(vectors?.settings ?? DEFAULT_VECTOR_SETTINGS));
  const stage = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; view: View; curtain: boolean } | null>(null);

  const width = vectors?.width ?? 1;
  const height = vectors?.height ?? 1;

  const fit = useCallback(() => {
    const box = stage.current;
    if (!box || !vectors) return;
    setView(fitView(vectors.width, vectors.height, box.clientWidth, box.clientHeight));
  }, [vectors]);

  // Ajusté à l'ouverture, et quand un nouveau dessin arrive (pas quand les
  // réglages changent : la vue reste sur le détail qu'on regardait).
  const framed = useRef<string | null>(null);
  useLayoutEffect(() => {
    if (!vectors) return;
    const key = `${vectors.width}x${vectors.height}|${vectors.imageUrl}`;
    if (framed.current === key) return;
    framed.current = key;
    fit();
  }, [vectors, fit]);

  useEffect(() => {
    if (vectors) setDraft(draftOf(vectors.settings));
  }, [vectors]);

  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const minScale = () => {
    const box = stage.current;
    if (!box) return 1e-3;
    return Math.min(1, fitView(width, height, box.clientWidth, box.clientHeight).scale / 2);
  };

  const zoomBy = (factor: number, cx?: number, cy?: number) => {
    const box = stage.current;
    if (!box || !view) return;
    setView(zoomAt(view, factor, cx ?? box.clientWidth / 2, cy ?? box.clientHeight / 2, minScale()));
  };

  const actualSize = () => {
    const box = stage.current;
    if (!box || !view) return;
    zoomBy(1 / view.scale);
  };

  const onWheel = (event: WheelEvent<HTMLDivElement>) => {
    const box = stage.current;
    if (!box || !view) return;
    const input = {
      deltaX: event.deltaX,
      deltaY: event.deltaY,
      deltaMode: event.deltaMode,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      wheelDeltaY: (event.nativeEvent as unknown as { wheelDeltaY?: number }).wheelDeltaY,
    };
    // Mêmes gestes que la visionneuse : pincement ou molette à crans pour
    // zoomer, deux doigts sur le pavé pour déplacer.
    const intent = classifyWheel(input);
    if (intent === "pan") {
      setView({ ...view, x: view.x - event.deltaX, y: view.y - event.deltaY });
      return;
    }
    const rect = box.getBoundingClientRect();
    setView(zoomAt(view, wheelZoomFactor(input, intent), event.clientX - rect.left, event.clientY - rect.top, minScale()));
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (!view) return;
    const onCurtain = (event.target as HTMLElement).classList.contains("vc-curtain-handle");
    drag.current = { x: event.clientX, y: event.clientY, view, curtain: onCurtain };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const start = drag.current;
    const box = stage.current;
    if (!start || !box) return;
    if (start.curtain) {
      const rect = box.getBoundingClientRect();
      setCurtain(Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width)));
      return;
    }
    setView({ ...start.view, x: start.view.x + event.clientX - start.x, y: start.view.y + event.clientY - start.y });
  };

  const onPointerUp = () => {
    drag.current = null;
  };

  const parsed = parseDraft(draft);
  const changed = vectors !== null && "settings" in parsed && !sameSettings(parsed.settings, vectors.settings);
  const verdict = vectors ? deviationVerdict(vectors) : null;
  const pixelated = view !== null && vectors !== null && view.scale >= 2;

  const layer = view
    ? { left: view.x, top: view.y, width: width * view.scale, height: height * view.scale }
    : { display: "none" as const };

  const stageWidth = stage.current?.clientWidth ?? 0;
  const curtainX = curtain * stageWidth;
  // Rideau : l'image à gauche, les vecteurs à droite (découpe à l'écran).
  const rightClip = `inset(0 0 0 ${Math.max(0, curtainX - (view?.x ?? 0))}px)`;

  return (
    <div className="vc" role="dialog" aria-label={`Comparer image et vecteurs : ${name}`}>
      <Toolbar label="Comparaison">
        <strong className="vc-title" title={name}>
          {name}
        </strong>
        <ToolSeparator />
        <Segmented label="Affichage" value={mode} onChange={setMode} options={MODES} />
        <ToolSpacer />
        <ToolGroup label="Fond">
          <Segmented label="Fond derrière le dessin" value={backdrop} onChange={onBackdrop} options={BACKDROPS} />
        </ToolGroup>
        <ToolSeparator />
        <ToolGroup label="Zoom">
          <IconButton icon="minus" label="Zoom arrière" onClick={() => zoomBy(1 / 1.5)} />
          <span className="vc-scale">{view ? scaleLabel(view.scale) : "—"}</span>
          <IconButton icon="plus" label="Zoom avant" onClick={() => zoomBy(1.5)} />
          <Button icon="fit" onClick={fit}>
            Ajuster
          </Button>
          <Button onClick={actualSize} title="Un pixel du dessin par pixel d'écran">
            100 %
          </Button>
        </ToolGroup>
        <ToolSeparator />
        <Button icon="close" onClick={onClose} title="Revenir à la bibliothèque (Échap)">
          Fermer
        </Button>
      </Toolbar>

      <div className="vc-body">
        <div
          className={`vc-stage fond-${backdrop}`}
          ref={stage}
          onWheel={onWheel}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
        >
          {vectors ? (
            <>
              <div className="vc-layer" style={layer}>
                {mode !== "vecteurs" && (
                  <img
                    src={vectors.imageUrl}
                    alt=""
                    draggable={false}
                    className={pixelated ? "vc-pixels" : undefined}
                    style={mode === "contour" ? { opacity: 0.85 } : undefined}
                  />
                )}
                {mode === "vecteurs" && <img src={vectors.filledUrl} alt="" draggable={false} />}
                {mode === "contour" && <img src={vectors.outlineUrl} alt="" draggable={false} />}
              </div>
              {mode === "rideau" && (
                <>
                  <div className={`vc-layer vc-curtain-right fond-${backdrop}`} style={{ ...layer, clipPath: rightClip }}>
                    <img src={vectors.filledUrl} alt="" draggable={false} />
                  </div>
                  <div className="vc-curtain-line" style={{ left: curtainX }}>
                    <span className="vc-curtain-handle" title="Glisser pour déplacer le rideau" />
                  </div>
                </>
              )}
            </>
          ) : (
            <p className="vc-wait">{error ?? "Vectorisation du dessin…"}</p>
          )}
          {busy && vectors && <p className="vc-busy">Nouveaux vecteurs en cours…</p>}
        </div>

        <Panel side="right" title="Vectorisation">
          <Section>
            {vectors ? (
              <dl className="ui-facts">
                <dt>Contours</dt>
                <dd>
                  {frNumber(vectors.stats.contours)}
                  {vectors.stats.removedContours > 0 && ` (${frNumber(vectors.stats.removedContours)} retirés)`}
                </dd>
                <dt>Morceaux</dt>
                <dd>
                  {frNumber(vectors.stats.lines + vectors.stats.cubics)} ({frNumber(vectors.stats.lines)} droites,{" "}
                  {frNumber(vectors.stats.cubics)} courbes)
                </dd>
                <dt>Écart maximal</dt>
                <dd className={verdict?.good ? "vc-bon" : "vc-mauvais"}>{frNumber(vectors.comparison.maxDeviationPx, 2)} px</dd>
                <dt>Pixels différents</dt>
                <dd>
                  {frNumber(vectors.comparison.mismatchedPixels)} sur {frNumber(vectors.comparison.sourcePixels)}
                </dd>
                <dt>Taille</dt>
                <dd>
                  {frNumber(vectors.width)} × {frNumber(vectors.height)} px
                </dd>
                <dt>Calcul</dt>
                <dd>{frNumber(vectors.seconds, 1)} s</dd>
              </dl>
            ) : (
              <p className="vc-note">…</p>
            )}
            {verdict && <p className="vc-note">{verdict.text}</p>}
          </Section>

          <Section title={vectors && !vectors.defaultSettings ? "Réglages propres à ce dessin" : "Réglages (par défaut)"}>
            <form
              onSubmit={(event) => {
                event.preventDefault();
                if (changed && "settings" in parsed) onApply(isDefault(parsed.settings) ? null : parsed.settings);
              }}
            >
              {FIELDS.map((field) => (
                <Property key={field.key} label={`${field.label} (${field.unit})`} hint={field.range ? field.help : undefined}>
                  <input
                    className={field.range ? "ui-input vc-threshold" : "ui-input"}
                    type="text"
                    inputMode="decimal"
                    title={field.help}
                    aria-label={field.label}
                    value={draft[field.key]}
                    disabled={!editable || busy}
                    onChange={(event) => setDraft({ ...draft, [field.key]: event.target.value })}
                  />
                  {field.range && (
                    <input
                      type="range"
                      min={field.range.min}
                      max={field.range.max}
                      step={field.range.step}
                      value={rangeValue(draft[field.key], field.range.fallback)}
                      disabled={!editable || busy}
                      onChange={(event) => setDraft({ ...draft, [field.key]: sliderText(Number(event.target.value)) })}
                      aria-label={field.label}
                    />
                  )}
                </Property>
              ))}
              {"error" in parsed && <p className="vc-erreur">{parsed.error}</p>}
              {error && vectors && <p className="vc-erreur">{error}</p>}
              <div className="vc-actions">
                <Button type="submit" variant="primary" disabled={!editable || busy || !changed}>
                  Appliquer
                </Button>
                <Button disabled={!editable || busy || !vectors || vectors.defaultSettings} onClick={() => onApply(null)}>
                  Réglages par défaut
                </Button>
              </div>
              {!editable && <p className="vc-note">Dans l'app seulement : ici, les vecteurs sont un exemple.</p>}
            </form>
          </Section>
          <Section>
            <p className="vc-note">
              Molette ou pincement : zoom. Glisser : déplacer. À partir de 200 %, les pixels de l'image se voient tels
              quels ; le contour rouge doit longer le bord du trait.
            </p>
          </Section>
        </Panel>
      </div>
    </div>
  );
}
