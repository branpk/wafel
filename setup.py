from glob import glob
from setuptools import setup, Extension
import subprocess
from shutil import copyfile

subprocess.run(['cargo', 'build', '--release', '--manifest-path', 'ext_modules/core/Cargo.toml'], check=True)
copyfile('ext_modules/core/target/release/core.dll', 'ext_modules/core.pyd')

setup(
  name = 'ext-modules',
  ext_modules = [
    Extension(
      'ext_modules.util',
      glob('ext_modules/util/**/*.c', recursive=True) +
        glob('ext_modules/util/**/*.cpp', recursive=True),
      include_dirs=['ext_modules', 'lib/pybind11/include'],
    ),
    Extension(
      'ext_modules.graphics',
      glob('ext_modules/graphics/**/*.c', recursive=True) +
        glob('ext_modules/graphics/**/*.cpp', recursive=True) +
        ['lib/gl/glad.c'],
      include_dirs=['ext_modules', 'lib/gl', 'lib/glm', 'lib/pybind11/include'],
    ),
  ],
)
