"""
Written by Dietrich
Usage: python time.py number_of_iterations frame_count [bin_commands ...]
"""

import sys
import random
import subprocess
import time
import typing
import datetime

def help():
  print("Usage: python time.py number_of_iterations frame_count")

# Get the result of a single example
def process_example(example : str, frame_count : int, file : "typing.IO"):
  output = subprocess.run(f"cargo run --release --features spirv --quiet --example {example} -- {frame_count}", capture_output=True, encoding="utf8")
  if output.returncode != 0:
    print(f"ERROR: {example} returned code {output.returncode}")
    sys.exit(output.returncode)

  file.write("---" + example + "---\n")
  for line in output.stdout.splitlines():
    stripped = line.strip()
    if len(stripped) == 0:
      continue
    file.write(f"{stripped}\n")

def main():
  if len(sys.argv) < 3 or not sys.argv[1].isdigit() or not sys.argv[2].isdigit():
    help()
    return
  examples = []
  with open("examples.txt", "r") as f:
    for line in f:
      examples.append(line.strip())
  examples *= int(sys.argv[1])
  # random.shuffle(examples)
  filename = datetime.datetime.now().strftime(f"%Y-%m-%d-%H-%M")
  with open("results/" + filename + ".result", "w") as file:
    for example in examples:
      print(f"Processing {example} ({sys.argv[2]} frames)")
      process_example(example, sys.argv[2], file)

if __name__ == "__main__":
  main()
