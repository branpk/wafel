import sys

min_python_version = (3, 8)
if sys.version_info < min_python_version:
  sys.stderr.write('Python >=%d.%d is required\n' % min_python_version)
  sys.stderr.flush()
  sys.exit(1)

import os

from wafel.config import config
from wafel.core import load_libsm64

if getattr(sys, 'frozen', False):
  print('Frozen') # TODO
else:
  script_dir = os.path.dirname(os.path.abspath(__file__))
  config._lib_directory = os.path.join(script_dir, 'lib')
  config._cache_directory = os.path.join(script_dir, '.wafel_cache')

load_libsm64(os.path.join(config.lib_directory, 'libsm64', 'sm64_us.dll'))

# from wafel.main import run

# run()
