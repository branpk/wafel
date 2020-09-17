import subprocess
import sys
import shutil
import os
from glob import glob

import wafel.config as config

if 'clean' in sys.argv[1:]:
  build_files = ['dist', 'build', 'wafel_core/target', 'wafel_core.pyd']
  for file in build_files:
    if os.path.isfile(file):
      print('Removing ' + file)
      os.remove(file)
    elif os.path.isdir(file):
      print('Removing ' + file)
      shutil.rmtree(file)

if sys.argv[1:] == [] or 'dist' in sys.argv[1:]:
  subprocess.run(
    ['cargo', 'build', '--release', '--manifest-path', 'wafel_core/Cargo.toml'],
    check=True,
  )
  shutil.copyfile('wafel_core/target/release/wafel_core.dll', 'wafel_core.pyd')

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
      '--name', 'wafel',
      'run.py',
    ],
    check=True,
  )

  shutil.copytree('assets', 'build/dist/assets')
  shutil.copytree('libsm64', 'build/dist/libsm64')

  print('Creating zip file')
  shutil.make_archive(
    'build/wafel_' + config.version_str('_'),
    'zip',
    'build/dist',
  )
