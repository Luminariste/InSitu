#!/usr/bin/env python3
"""Mise à jour automatique (D16) : outils de la fabrication.

  mise_a_jour.py verifier FICHIER.sig CONF.json
      Vérifie que la signature de l'archive a été faite avec la clé dont la
      clé publique est dans tauri.conf.json (même identifiant de clé). Attrape
      un secret collé de travers avant de publier une mise à jour que les apps
      installées refuseraient.

  mise_a_jour.py manifeste VERSION FICHIER.sig URL SORTIE.json
      Écrit le manifeste `latest.json` lu par l'app au lancement.
"""
import base64
import datetime
import json
import sys


def minisign_blob(text_b64: str) -> bytes:
    """Contenu binaire d'une clé publique ou d'une signature minisign, telles
    que Tauri les écrit (texte minisign lui-même encodé en base64)."""
    text = base64.b64decode(text_b64.strip()).decode("utf-8")
    lines = [line for line in text.splitlines() if line.strip()]
    # Ligne 1 : commentaire ; ligne 2 : données (algorithme 2 octets, puis
    # identifiant de clé 8 octets, puis clé ou signature).
    return base64.b64decode(lines[1])


def key_id(text_b64: str) -> bytes:
    return minisign_blob(text_b64)[2:10]


def verifier(sig_path: str, conf_path: str) -> None:
    with open(conf_path, encoding="utf-8") as f:
        pubkey = json.load(f)["plugins"]["updater"]["pubkey"]
    with open(sig_path, encoding="utf-8") as f:
        sig = f.read()
    expected, actual = key_id(pubkey), key_id(sig)
    if expected != actual:
        sys.exit(
            f"La signature a été faite avec la clé {actual[::-1].hex().upper()}, mais l'app attend "
            f"la clé {expected[::-1].hex().upper()} : le secret TAURI_SIGNING_PRIVATE_KEY ne correspond "
            "pas à la clé publique de tauri.conf.json."
        )
    print(f"Signature faite avec la bonne clé ({actual[::-1].hex().upper()}).")


def manifeste(version: str, sig_path: str, url: str, out_path: str) -> None:
    with open(sig_path, encoding="utf-8") as f:
        signature = f.read().strip()
    data = {
        "version": version,
        "notes": f"InSitu {version}",
        "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": {"darwin-aarch64": {"signature": signature, "url": url}},
    }
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print(f"Manifeste écrit : {out_path}")


if __name__ == "__main__":
    command, *args = sys.argv[1:]
    {"verifier": verifier, "manifeste": manifeste}[command](*args)
