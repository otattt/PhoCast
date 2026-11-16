import os
from PIL import Image, ImageOps

source_path = r"C:\Users\Dou\.gemini\antigravity\brain\61617565-de09-4417-bf68-f61b12092b5b\concept_b_stream_shutter_icon_1794837900823.jpg"
repo_root = r"C:\Users\Dou\.gemini\antigravity\scratch\PhoCast"

img = Image.open(source_path).convert("RGBA")

# Ensure assets directories exist
assets_dir = os.path.join(repo_root, "assets")
os.makedirs(assets_dir, exist_ok=True)

# 1. Save master PNG
master_png = os.path.join(assets_dir, "icon.png")
img.save(master_png, "PNG")
print(f"Saved master icon: {master_png}")

# 2. Generate Windows .ico with multiple resolutions
ico_sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]
rust_assets_dir = os.path.join(repo_root, "client-pc", "rust", "assets")
os.makedirs(rust_assets_dir, exist_ok=True)
rust_ico_path = os.path.join(rust_assets_dir, "icon.ico")
installer_ico_path = os.path.join(repo_root, "installer", "icon.ico")

img.save(rust_ico_path, format="ICO", sizes=ico_sizes)
img.save(installer_ico_path, format="ICO", sizes=ico_sizes)
print(f"Saved Windows ICO: {rust_ico_path} and {installer_ico_path}")

# 3. Generate Android Mipmap icons
android_res = os.path.join(repo_root, "android", "app", "src", "main", "res")
android_densities = {
    "mipmap-mdpi": 48,
    "mipmap-hdpi": 72,
    "mipmap-xhdpi": 96,
    "mipmap-xxhdpi": 144,
    "mipmap-xxxhdpi": 192,
}

# Create circular mask for round icons
def make_round(im, size):
    im_resized = im.resize((size, size), Image.Resampling.LANCZOS)
    mask = Image.new("L", (size, size), 0)
    from PIL import ImageDraw
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, size, size), fill=255)
    rounded = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    rounded.paste(im_resized, (0, 0), mask)
    return rounded

for folder, size in android_densities.items():
    target_folder = os.path.join(android_res, folder)
    os.makedirs(target_folder, exist_ok=True)
    
    # Square / Squircle
    sq_icon = img.resize((size, size), Image.Resampling.LANCZOS)
    sq_path = os.path.join(target_folder, "ic_launcher.png")
    sq_icon.save(sq_path, "PNG")
    
    # Round
    round_icon = make_round(img, size)
    round_path = os.path.join(target_folder, "ic_launcher_round.png")
    round_icon.save(round_path, "PNG")
    
    print(f"Saved Android {folder} ({size}x{size})")

print("All icons successfully generated!")
