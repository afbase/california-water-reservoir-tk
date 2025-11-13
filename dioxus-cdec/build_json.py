#!/usr/bin/env python3
import json
import subprocess
import os

# Extract the cumulative data
print("Extracting data from tar.lzma...")
result = subprocess.run(
    ['xz', '-dc', '../fixtures/cumulative_v2.tar.lzma'],
    capture_output=True
)
tar_data = result.stdout
result2 = subprocess.run(
    ['tar', '-xO'],
    input=tar_data,
    capture_output=True
)

csv_data = result2.stdout.decode('utf-8')

# Convert to JSON
print("Converting to JSON...")
lines = csv_data.strip().split('\n')
data = []
for line in lines:
    if line:
        parts = line.split(',')
        if len(parts) == 2:
            date_str = parts[0]
            # Convert YYYYMMDD to YYYY-MM-DD
            formatted_date = f"{date_str[0:4]}-{date_str[4:6]}-{date_str[6:8]}"
            water_level = int(parts[1])
            data.append([formatted_date, water_level])

json_data = {"observations": data}

# Write JSON
json_path = 'data/reservoir_data.json'
with open(json_path, 'w') as f:
    json.dump(json_data, f, separators=(',', ':'))

print(f"Created JSON with {len(data)} records")

# Compress with zstd
print("Compressing JSON with zstd...")
subprocess.run([
    'zstd', '-19', '-f',
    json_path,
    '-o', 'data/reservoir_data.json.zst'
])

# Get file sizes
json_size = os.path.getsize(json_path)
compressed_size = os.path.getsize('data/reservoir_data.json.zst')

print(f"JSON size: {json_size:,} bytes")
print(f"Compressed size: {compressed_size:,} bytes")
print(f"Compression ratio: {compressed_size/json_size*100:.2f}%")
print("Done!")
