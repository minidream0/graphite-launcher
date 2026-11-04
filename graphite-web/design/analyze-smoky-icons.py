"""Read alpha bounds only; never edit, crop or resave the generated PNG assets."""
import json
from pathlib import Path
from PIL import Image

root = Path(__file__).resolve().parents[1] / 'public' / 'smoky-icons'
sheets = []
for filename, group, offset in [('categories', 'categories', 0), ('actions-a', 'actions', 0), ('actions-b', 'actions', 16), ('states-a', 'states', 0), ('states-b', 'states', 16)]:
    file = root / (filename + '.png')
    if not file.exists():
        continue
    image = Image.open(file)
    assert image.mode == 'RGBA', f'{filename}: missing alpha'
    assert image.getchannel('A').getextrema() == (0, 255)
    width, height = image.size
    pixels = image.load()
    def cuts(vertical):
        length, other = (height, width) if vertical else (width, height)
        density = [sum(pixels[k, i][3] >= 160 if vertical else pixels[i, k][3] >= 160 for k in range(other)) for i in range(length)]
        result = [0]
        for index in range(1, 4):
            target = index * length // 4
            left, right = target - length // 12, target + length // 12
            minimum = min(density[left:right])
            runs, run = [], []
            for pos in range(left, right):
                if density[pos] == minimum:
                    run.append(pos)
                elif run:
                    runs.append(run); run = []
            if run: runs.append(run)
            best = min(runs, key=lambda r: abs((r[0]+r[-1])/2-target))
            result.append((best[0]+best[-1])//2)
        return result+[length]
    rows, cols = cuts(True), cuts(False)
    frames = []
    for row in range(4):
        for col in range(4):
            left, right = cols[col], cols[col+1]
            top, bottom = rows[row], rows[row+1]
            min_x, min_y, max_x, max_y, count = right, bottom, left, top, 0
            for y in range(top, bottom):
                for x in range(left, right):
                    if pixels[x, y][3] >= 128:
                        min_x, min_y = min(min_x, x), min(min_y, y)
                        max_x, max_y = max(max_x, x), max(max_y, y)
                        count += 1
            assert count > 500, f'{filename} cell {row},{col}: missing icon'
            assert min_x > left and min_y > top and max_x < right - 1 and max_y < bottom - 1, f'{filename} cell {row},{col}: clipped icon'
            x, y = max(left, min_x - 3), max(top, min_y - 3)
            frames.append([x, y, min(right, max_x + 4) - x, min(bottom, max_y + 4) - y])
    sheets.append(dict(group=group, offset=offset, file=file.name, width=width, height=height, frames=frames))
print(json.dumps(dict(sheets=sheets), separators=(',', ':')))
