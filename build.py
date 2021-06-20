import subprocess
import sys
import shutil
import os

import wafel.config as config
from wafel.game_versions import lock_game_version, unlocked_game_versions, find_locked_dlls

config.init()

if 'clean' in sys.argv[1:]:
  build_files = ['dist', 'build', 'target', 'wafel_core.pyd']
  for file in build_files:
    if os.path.isfile(file):
      print('Removing ' + file)
      os.remove(file)
    elif os.path.isdir(file):
      print('Removing ' + file)
      shutil.rmtree(file)

if sys.argv[1:] == [] or 'dist' in sys.argv[1:]:
  subprocess.run(
    ['cargo', '+nightly', 'build', '--release'],
    check=True,
  )
  shutil.copyfile('target/release/wafel_core.dll', 'wafel_core.pyd')

if 'lock' in sys.argv[1:]:
  print('Locking DLLs')
  for game_version in unlocked_game_versions():
    name = 'sm64_' + game_version.lower()
    lock_game_version(
      game_version,
      os.path.join('roms', name + '.z64'),
      os.path.join('libsm64', name + '.dll'),
      os.path.join('libsm64', name + '.dll.locked'),
    )

if 'dist' in sys.argv[1:]:
  shutil.rmtree('build/dist', ignore_errors=True)

  # TODO: Need to ensure it runs on nightly
  subprocess.run(
    [sys.executable, 'setup.py', 'bdist_wheel'],
    check=True,
  )

  import ctypes
  import ctypes.util
  from glfw.library import glfw, msvcr

  subprocess.run(
    [
      'pyinstaller',
      '--onefile',
      '--icon', '../wafel.ico',
      '--noconsole',
      '--specpath', 'build',
      '--distpath', 'build/dist',
      '--add-binary', os.pathsep.join([ctypes.util.find_library('msvcp140.dll'), '.']),
      '--add-binary', os.pathsep.join([glfw._name, '.']),
      '--add-binary', os.pathsep.join([msvcr._name, '.']),
      '--name', 'wafel',
      'run.py',
    ],
    check=True,
  )

  print('Copying locked DLLs')
  os.makedirs(os.path.join('build', 'dist', 'libsm64'))
  for game_version, locked_dll in find_locked_dlls().items():
    name = 'sm64_' + game_version.lower()
    shutil.copyfile(
      locked_dll,
      os.path.join('build', 'dist', 'libsm64', name + '.dll.locked')
    )

  print('Copying tools')
  os.makedirs('build/dist/tools')
  shutil.copyfile('target/release/libsm64_layout.exe', 'build/dist/tools/libsm64_layout.exe')
  shutil.copyfile('target/release/libsm64_lock.exe', 'build/dist/tools/libsm64_lock.exe')

  print('Copying bindings')
  os.makedirs('build/dist/bindings')
  shutil.copytree('dist', 'build/dist/bindings/python')

  print('Copying .pdb')
  shutil.copyfile('target/release/wafel_core.pdb', 'build/dist/wafel_core.pdb')
