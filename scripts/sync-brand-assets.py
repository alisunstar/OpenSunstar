"""Sync OpenSunstar brand assets from docs/LOGO素材 (transparent PNG preferred)."""

from __future__ import annotations

import os
import shutil

from PIL import Image

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOGO_DIR = os.path.join(REPO, "docs", "LOGO素材")


def pick(*needles: str) -> str:
    for name in sorted(os.listdir(LOGO_DIR)):
        if not name.endswith(".png"):
            continue
        for needle in needles:
            if needle in name:
                return os.path.join(LOGO_DIR, name)
    raise FileNotFoundError(f"No logo file matching {needles!r} in {LOGO_DIR}")


def save_resize(src: str, dest: str, size: int | tuple[int, int]) -> None:
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    im = Image.open(src).convert("RGBA")
    target = (size, size) if isinstance(size, int) else size
    im.resize(target, Image.Resampling.LANCZOS).save(dest, optimize=True)
    print(f"wrote {os.path.relpath(dest, REPO)} {target[0]}x{target[1]}")


def main() -> None:
    transparent = pick("透明")

    # Master icon for `pnpm tauri icon` (upscale transparent source to 1024)
    master_out = os.path.join(REPO, "app-icon.png")
    save_resize(transparent, master_out, 1024)

    save_resize(transparent, os.path.join(REPO, "src/assets/icons/app-icon.png"), 512)
    save_resize(transparent, os.path.join(REPO, "src/assets/icons/app-icon-32.png"), 32)
    save_resize(transparent, os.path.join(REPO, "src/assets/icons/app-icon-64.png"), 64)
    save_resize(transparent, os.path.join(REPO, "src/assets/icons/app-icon-128.png"), 128)
    save_resize(transparent, os.path.join(REPO, "src/assets/icons/app-icon-256.png"), 256)

    web_assets = os.path.join(REPO, "website/assets")
    save_resize(transparent, os.path.join(web_assets, "icon.png"), 512)
    # 2x assets for crisp display at 32 / 22 CSS px
    save_resize(transparent, os.path.join(web_assets, "logo-nav.png"), 64)
    save_resize(transparent, os.path.join(web_assets, "logo-sm.png"), 44)

    tray_dir = os.path.join(REPO, "src-tauri/icons/tray/macos")
    save_resize(transparent, os.path.join(tray_dir, "statusbar_template_3x.png"), 72)
    save_resize(transparent, os.path.join(tray_dir, "statusTemplate.png"), 22)
    save_resize(transparent, os.path.join(tray_dir, "statusTemplate@2x.png"), 44)

    print("Brand asset sync complete (transparent PNG).")


if __name__ == "__main__":
    main()
