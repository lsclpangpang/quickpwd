"""生成应用图标（锁形，深色圆角底 + 蓝色渐变）。用法: python tools/make_icons.py"""
import os
from PIL import Image, ImageDraw

OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")
S = 1024


def draw():
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    # 渐变底
    grad = Image.new("RGBA", (S, S))
    gd = ImageDraw.Draw(grad)
    top, bot = (110, 150, 255), (72, 92, 230)
    for y in range(S):
        t = y / (S - 1)
        gd.line([(0, y), (S, y)], fill=tuple(int(a + (b - a) * t) for a, b in zip(top, bot)) + (255,))
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([40, 40, S - 40, S - 40], radius=220, fill=255)
    img.paste(grad, (0, 0), mask)
    d = ImageDraw.Draw(img)
    white = (255, 255, 255, 255)
    # 锁梁
    d.arc([330, 200, 694, 564], start=180, end=360, fill=white, width=78)
    d.rectangle([330, 380, 408, 470], fill=white)
    d.rectangle([616, 380, 694, 470], fill=white)
    # 锁体
    d.rounded_rectangle([262, 450, 762, 830], radius=70, fill=white)
    # 锁孔
    hole = (88, 118, 245, 255)
    d.ellipse([472, 560, 552, 640], fill=hole)
    d.rounded_rectangle([494, 610, 530, 720], radius=18, fill=hole)
    return img


def main():
    os.makedirs(OUT, exist_ok=True)
    img = draw()
    for name, size in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)):
        img.resize((size, size), Image.LANCZOS).save(os.path.join(OUT, name))
    img.save(os.path.join(OUT, "icon.ico"),
             sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])


if __name__ == "__main__":
    main()
