import sys

min_python_version = (3, 8)
if sys.version_info < min_python_version:
  sys.stderr.write('Python >=%d.%d is required\n' % min_python_version)
  sys.stderr.flush()
  sys.exit(1)


import os

from wafel.config import config
from wafel.main import run

if getattr(sys, 'frozen', False):
  exe_dir = os.path.dirname(sys.executable)
  config._assets_directory = os.path.join(exe_dir, 'assets')
  config._lib_directory = exe_dir
  config._cache_directory = os.path.join(exe_dir, '.wafel_cache')
else:
  script_dir = os.path.dirname(os.path.abspath(__file__))
  config._assets_directory = os.path.join(script_dir, 'assets')
  config._lib_directory = os.path.join(script_dir, 'lib')
  config._cache_directory = os.path.join(script_dir, '.wafel_cache')

run()
