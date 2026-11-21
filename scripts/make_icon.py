"""生成应用图标（马卡龙渐变 + 白色表盘）。"""
from PIL import Image, ImageDraw, ImageFilter

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))

# 渐变底
grad = Image.new("RGB", (S, S))
gd = ImageDraw.Draw(grad)
for y in range(S):
    for_x = y / S
    r = int(255 * (1 - 0.18 * for_x))
    g = int(163 + 40 * for_x)
    b = int(184 + 64 * for_x)
    gd.line([(0, y), (S, y)], fill=(r, g, b))

# 斜向叠加一层蓝紫，让渐变更“马卡龙”
overlay = Image.new("RGBA", (S, S), (0, 0, 0, 0))
od = ImageDraw.Draw(overlay)
for i in range(S):
    t = i / S
    od.line([(i, 0), (i, S)], fill=(159, 211, 255, int(40 + 50 * t)))
grad = Image.alpha_composite(grad.convert("RGBA"), overlay)

mask = Image.new("L", (S, S), 0)
md = ImageDraw.Draw(mask)
md.rounded_rectangle([0, 0, S - 1, S - 1], radius=232, fill=255)
img.paste(grad, (0, 0), mask)

d = ImageDraw.Draw(img)
cx = cy = S / 2
ring_r = 300
d.ellipse(
    [cx - ring_r, cy - ring_r, cx + ring_r, cy + ring_r],
    outline=(255, 255, 255, 235),
    width=54,
)

# 指针
d.line([(cx, cy - 20), (cx, cy - 185)], fill=(255, 255, 255, 255), width=48)
d.line([(cx, cy), (cx + 118, cy + 96)], fill=(255, 255, 255, 255), width=48)
for (x, y) in [(cx, cy - 185), (cx + 118, cy + 96)]:
    d.ellipse([x - 24, y - 24, x + 24, y + 24], fill=(255, 255, 255, 255))
d.ellipse([cx - 34, cy - 34, cx + 34, cy + 34], fill=(255, 255, 255, 255))
d.ellipse([cx - 14, cy - 14, cx + 14, cy + 14], fill=(255, 163, 184, 255))

# 柔和内高光
glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(glow).ellipse([-260, -360, 620, 300], fill=(255, 255, 255, 26))
img = Image.alpha_composite(img, glow.filter(ImageFilter.GaussianBlur(90)))

img.save("src-tauri/icons/icon.png")
img.resize((512, 512), Image.LANCZOS).save("src-tauri/icons/512x512.png")
img.resize((256, 256), Image.LANCZOS).save("src-tauri/icons/128x128@2x.png")
img.resize((128, 128), Image.LANCZOS).save("src-tauri/icons/128x128.png")
img.resize((32, 32), Image.LANCZOS).save("src-tauri/icons/32x32.png")
img.resize((256, 256), Image.LANCZOS).save(
    "src-tauri/icons/icon.ico",
    format="ICO",
    sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
)
print("icons written")
