import subprocess
import sys
import shutil
import os
from glob import glob

from wafel.config import VERSION

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

if sys.argv[1:] == [] or 'dist' in sys.argv[1:]:
  subprocess.run([sys.executable, 'setup.py', 'develop'], check=True)

if 'dist' in sys.argv[1:]:
  shutil.rmtree('build/dist', ignore_errors=True)

  from glfw.library import glfw

  subprocess.run(
    [
      'pyinstaller',
      '--onefile',
      '--noconsole',
      '--specpath', 'build',
      '--distpath', 'build/dist',
      '--add-binary', glfw._name + os.pathsep + '.',
      '--name', 'wafel_' + '_'.join(map(str, VERSION)),
      'run.py',
    ],
    check=True,
  )

  shutil.copytree('assets', 'build/dist/assets')
  shutil.copytree('lib/libsm64', 'build/dist/libsm64')
