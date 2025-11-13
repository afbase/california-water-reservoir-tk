#!/usr/bin/env python3
import sqlite3
import csv
import subprocess
import os
import io

def extract_tar_lzma(file_path):
    """Extract data from tar.lzma file"""
    result = subprocess.run(
        ['xz', '-dc', file_path],
        capture_output=True
    )
    if result.returncode != 0:
        print(f"Error extracting {file_path}")
        exit(1)

    tar_data = result.stdout
    result2 = subprocess.run(
        ['tar', '-xO'],
        input=tar_data,
        capture_output=True
    )

    return result2.stdout.decode('utf-8')

# Create SQLite database
print("Creating comprehensive SQLite database...")
db_path = 'data/reservoir_data.db'
if os.path.exists(db_path):
    os.remove(db_path)

conn = sqlite3.connect(db_path)
cursor = conn.cursor()

# Create statewide_observations table
print("\n1. Creating statewide_observations table...")
cursor.execute('''
    CREATE TABLE statewide_observations (
        date TEXT PRIMARY KEY,
        water_level INTEGER NOT NULL
    )
''')
cursor.execute('CREATE INDEX idx_statewide_date ON statewide_observations(date)')

# Load cumulative statewide data
print("   Loading cumulative statewide data...")
csv_data = extract_tar_lzma('../fixtures/cumulative_v2.tar.lzma')
lines = csv_data.strip().split('\n')
statewide_data = []
for line in lines:
    if line:
        parts = line.split(',')
        if len(parts) == 2:
            date_str = parts[0]
            # Convert YYYYMMDD to YYYY-MM-DD
            formatted_date = f"{date_str[0:4]}-{date_str[4:6]}-{date_str[6:8]}"
            water_level = int(parts[1])
            statewide_data.append((formatted_date, water_level))

cursor.executemany('INSERT INTO statewide_observations VALUES (?, ?)', statewide_data)
print(f"   Inserted {len(statewide_data)} statewide observations")

# Create reservoirs table
print("\n2. Creating reservoirs table...")
cursor.execute('''
    CREATE TABLE reservoirs (
        station_id TEXT PRIMARY KEY,
        dam_name TEXT,
        lake_name TEXT,
        stream_name TEXT,
        capacity INTEGER,
        year_fill INTEGER
    )
''')

# Load reservoir metadata from capacity.csv
print("   Loading reservoir metadata...")
with open('../fixtures/capacity.csv', 'r') as f:
    reader = csv.DictReader(f)
    reservoirs_data = []
    for row in reader:
        station_id = row['ID']
        dam_name = row['DAM']
        lake_name = row['LAKE']
        stream_name = row['STREAM']
        capacity = int(row['CAPACITY (AF)']) if row['CAPACITY (AF)'] else None
        year_fill = int(row['YEAR FILL']) if row['YEAR FILL'] else None
        reservoirs_data.append((station_id, dam_name, lake_name, stream_name, capacity, year_fill))

cursor.executemany('INSERT INTO reservoirs VALUES (?, ?, ?, ?, ?, ?)', reservoirs_data)
print(f"   Inserted {len(reservoirs_data)} reservoirs")

# Create reservoir_observations table
print("\n3. Creating reservoir_observations table...")
cursor.execute('''
    CREATE TABLE reservoir_observations (
        station_id TEXT NOT NULL,
        date TEXT NOT NULL,
        water_level INTEGER NOT NULL,
        PRIMARY KEY (station_id, date),
        FOREIGN KEY (station_id) REFERENCES reservoirs(station_id)
    )
''')
cursor.execute('CREATE INDEX idx_reservoir_station ON reservoir_observations(station_id)')
cursor.execute('CREATE INDEX idx_reservoir_date ON reservoir_observations(date)')

# Load per-reservoir observations
print("   Loading per-reservoir observations...")
csv_data = extract_tar_lzma('../fixtures/reservoirs.tar.lzma')
reader = csv.reader(io.StringIO(csv_data))

reservoir_obs_data = []
count = 0
for row in reader:
    if len(row) < 4:
        continue
    station_id = row[0]
    # Skip the second column (appears to be a code)
    date_str = row[2]
    # Convert YYYYMMDD to YYYY-MM-DD
    formatted_date = f"{date_str[0:4]}-{date_str[4:6]}-{date_str[6:8]}"
    water_level = int(row[3])
    reservoir_obs_data.append((station_id, formatted_date, water_level))
    count += 1

    # Batch insert every 10000 rows for performance
    if count % 10000 == 0:
        cursor.executemany('INSERT OR IGNORE INTO reservoir_observations VALUES (?, ?, ?)', reservoir_obs_data)
        print(f"   ... {count} observations processed")
        reservoir_obs_data = []

# Insert remaining rows
if reservoir_obs_data:
    cursor.executemany('INSERT OR IGNORE INTO reservoir_observations VALUES (?, ?, ?)', reservoir_obs_data)

print(f"   Inserted {count} reservoir observations")

conn.commit()
conn.close()

print(f"\nDatabase created with:")
print(f"  - {len(statewide_data)} statewide observations")
print(f"  - {len(reservoirs_data)} reservoirs")
print(f"  - {count} per-reservoir observations")

# Compress with zstd
print("\nCompressing database with zstd...")
subprocess.run([
    'zstd', '-19', '-f',
    db_path,
    '-o', 'data/reservoir_data.db.zst'
])

# Get file sizes
original_size = os.path.getsize(db_path)
compressed_size = os.path.getsize('data/reservoir_data.db.zst')

print(f"\nOriginal size: {original_size:,} bytes")
print(f"Compressed size: {compressed_size:,} bytes")
print(f"Compression ratio: {compressed_size/original_size*100:.2f}%")
print("Done!")
