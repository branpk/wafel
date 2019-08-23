from glob import glob
from setuptools import setup, Extension

ext_modules = [
  Extension(
    'ext_modules.graphics',
    glob('ext_modules/graphics/**/*.cpp', recursive=True) + ['lib/gl/glad.c'],
    include_dirs=['lib/gl', 'lib/glm', 'lib/libsm64/us'],
  ),
]

setup(
  name='ext-modules',
  ext_modules=ext_modules,
)
