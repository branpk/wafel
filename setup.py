from glob import glob
from setuptools import setup, Extension

ext_modules = [
  Extension(
    'ext_modules.graphics',
    glob('ext_modules/graphics/**/*.c*', recursive=True) + ['lib/gl/glad.c'],
    include_dirs=['lib/gl', 'lib/glm', 'lib/pybind11/include'],
  ),
]

setup(
  name='ext-modules',
  ext_modules=ext_modules,
)
