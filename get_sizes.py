import os
from pathlib import Path

def get_size(path):
    total = 0
    try:
        for entry in os.scandir(path):
            if entry.is_file(follow_symlinks=False):
                try:
                    total += entry.stat(follow_symlinks=False).st_size
                except:
                    pass
            elif entry.is_dir(follow_symlinks=False):
                total += get_size(entry.path)
    except PermissionError:
        pass
    return total

def main():
    root = Path.cwd()

    # Get all top-level items
    items = []
    for item in root.iterdir():
        if item.is_file():
            size_gb = item.stat().st_size / (1024**3)
            items.append((str(item.name), size_gb, 'file'))
        elif item.is_dir():
            size_gb = get_size(str(item)) / (1024**3)
            items.append((str(item.name), size_gb, 'dir'))

    # Sort by size descending
    items.sort(key=lambda x: x[1], reverse=True)

    # Print results
    print(f"{'Name':<50} {'Size (GB)':<15} {'Type':<10}")
    print("=" * 75)
    for name, size, item_type in items:
        print(f"{name:<50} {size:<15.6f} {item_type:<10}")

    # Print total
    total_gb = sum(item[1] for item in items)
    print("=" * 75)
    print(f"{'TOTAL':<50} {total_gb:<15.6f}")

if __name__ == "__main__":
    main()
