import subprocess
import sys
import shutil
import os
from glob import glob

if 'clean' in sys.argv[1:]:
  build_files = ['dist', 'ext_modules.egg-info', 'build']
  build_files += glob('ext_modules/**/*.pyd', recursive=True)
  for file in build_files:
    if os.path.isfile(file):
      print('Removing ' + file)
      os.remove(file)
    elif os.path.isdir(file):
      print('Removing ' + file)
      shutil.rmtree(file)

else:
  subprocess.run([sys.executable, 'setup.py', 'develop'], check=True)
