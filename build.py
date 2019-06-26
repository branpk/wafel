import subprocess
import sys
import shutil
import os

clutter = ['dist', 'ext_modules.egg-info', 'build']
if 'clean' not in sys.argv[1:]:
  clutter = list(filter(lambda d: not os.path.isdir(d), clutter))

subprocess.run([sys.executable, 'setup.py', 'install'])

for dir in clutter:
  print('Removing ' + dir)
  shutil.rmtree(dir, ignore_errors=True)
