#!/usr/bin/env python

import os, sys
import bisect

INPUT ="massif.out"
SNAPSHOT_PREFIX = "trees/massif"
ROOT_LINES_INDEX = "root-lines.txt"
SNAPSHOT_LINES_INDEX = "snapshot-lines.txt"

if not os.path.exists(ROOT_LINES_INDEX):
    print(f"First, generate the 'root lines' index by running the following command:", file=sys.stderr)
    print(f"    grep -n -E '^\w+:' {INPUT} > {ROOT_LINES_INDEX}", file=sys.stderr)
    sys.exit(1)
if not os.path.exists("snapshot-lines.txt"):
    print(f"Second, generate the 'snapshot lines' index by running the following command:", file=sys.stderr)
    print(f"    grep -n -E '^snapshot=' {INPUT} > {SNAPSHOT_LINES_INDEX}", file=sys.stderr)
    sys.exit(1)
sys.exit(0)

sorted_snapshots = []
with open(SNAPSHOT_LINES_INDEX) as fp:
    for line in fp:
        lineno, tail = line.split(':')
        _, snapshot = tail.rstrip().split('=')

        entry = (int(lineno), snapshot)
        i = bisect.bisect_left(sorted_snapshots, entry)
        sorted_snapshots.insert(i, entry)

sorted_snapshot_linenos = []
with open(ROOT_LINES_INDEX) as fp:
    for line in fp:
        lineno = int(line.split(':')[0])

        i = bisect.bisect_left(sorted_snapshots, (lineno, "undefined"))
        if i == 0: raise NotImplementedError

        # Each `[start_lineno, end_lineno]` entry is a line interval
        # between "snapshot=" lines.
        start_lineno, snapshot = sorted_snapshots[i-1]
        if i < len(sorted_snapshots):
            end_lineno = int(sorted_snapshots[i][0])
        else:
            end_lineno = "EOF"

        entry = (int(start_lineno), end_lineno, snapshot)
        i = bisect.bisect_left(sorted_snapshot_linenos, entry)
        sorted_snapshot_linenos.insert(i, entry)

def read_line_interval(lines, start_lineno, end_lineno):
    for lineno, _ in lines:
        if lineno >= start_lineno:
            break

    is_reading = False
    for lineno, line in lines:
        if line.startswith('n'):
            is_reading = True
        elif not is_reading:
            continue

        if line.startswith('#'):
            break

        yield lineno, line

with open(INPUT) as fp:
    lines = enumerate(fp, start=1)
    for start_lineno, end_lineno, snapshot in sorted_snapshot_linenos:
        print(f"Reading data in {start_lineno}..{end_lineno} ... ", end='')
        with open(f"{SNAPSHOT_PREFIX}_snapshot-{snapshot}.out", 'w') as fp:
            for _, line in read_line_interval(lines, start_lineno, end_lineno):
                fp.write(line)
        print("done")
