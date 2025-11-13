#!/usr/bin/env python3
import sqlite3
import csv
import subprocess
import os

# Extract the cumulative data
print("Extracting data from tar.lzma...")
result = subprocess.run(
    ['xz', '-dc', '../fixtures/cumulative_v2.tar.lzma'],
    capture_output=True
)
if result.returncode != 0:
    print("Error extracting data")
    exit(1)

tar_data = result.stdout
result2 = subprocess.run(
    ['tar', '-xO'],
    input=tar_data,
    capture_output=True
)

csv_data = result2.stdout.decode('utf-8')

# Create SQLite database
print("Creating SQLite database...")
db_path = 'data/reservoir_data.db'
if os.path.exists(db_path):
    os.remove(db_path)

conn = sqlite3.connect(db_path)
cursor = conn.cursor()

# Create table
cursor.execute('''
    CREATE TABLE observations (
        date TEXT PRIMARY KEY,
        water_level INTEGER NOT NULL
    )
''')

# Create index on date
cursor.execute('CREATE INDEX idx_date ON observations(date)')

# Insert data
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
            data.append((formatted_date, water_level))

cursor.executemany('INSERT INTO observations VALUES (?, ?)', data)
conn.commit()
conn.close()

print(f"Created database with {len(data)} records")

# Compress with zstd
print("Compressing database with zstd...")
subprocess.run([
    'zstd', '-19', '-f',
    db_path,
    '-o', 'data/reservoir_data.db.zst'
])

# Get file sizes
original_size = os.path.getsize(db_path)
compressed_size = os.path.getsize('data/reservoir_data.db.zst')

print(f"Original size: {original_size:,} bytes")
print(f"Compressed size: {compressed_size:,} bytes")
print(f"Compression ratio: {compressed_size/original_size*100:.2f}%")
print("Done!")
