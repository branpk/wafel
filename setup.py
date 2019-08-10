from glob import glob
from setuptools import setup, Extension

ext_modules = [
  Extension(
    '_ext_modules.graphics',
    glob('ext_modules/graphics/**/*.c', recursive=True) + ['lib/gl/glad.c'],
    include_dirs=['lib/gl', 'lib/libsm64/us'],
  ),
]

setup(
  name='ext-modules',
  ext_modules=ext_modules,
)
