from glob import glob
from setuptools import setup, Extension

ext_modules = [
  Extension(
    'ext_modules.graphics',
    glob('ext_modules/graphics/**/*.cpp', recursive=True) + ['lib/gl/glad.c'],
    include_dirs=['lib/gl', 'lib/glm', 'lib/pybind11/include', 'lib/libsm64/jp'],
  ),
]

setup(
  name='ext-modules',
  ext_modules=ext_modules,
)
