"""Spawns a grandchild that would outlive us, then waits. Used to prove Stop kills the tree."""
import subprocess
import sys
import time

child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(600)"])
with open("grandchild.pid", "w") as f:
    f.write(str(child.pid))
print(f"spawned grandchild {child.pid}", flush=True)

if __name__ == "__main__":
    time.sleep(600)
