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
  print('Frozen') # TODO
else:
  script_dir = os.path.dirname(os.path.abspath(__file__))
  config._lib_directory = os.path.join(script_dir, 'lib')
  config._cache_directory = os.path.join(script_dir, '.wafel_cache')

# run()



from wafel.core import GameLib, load_libsm64, Variables, Variable
from wafel.core.timeline import _GameStateSequence
from wafel.format_m64 import load_m64
import time

lib = load_libsm64('jp')
variables = Variable.create_all(lib)
_, edits = load_m64('test_files/1key_j.m64')

sequence = _GameStateSequence(lib, variables, edits)

buffer = sequence.alloc_state_buffer()

start_time = time.time()
for i in range(3000):
  if i % 10 == 0:
    sequence.raw_copy_state(buffer, sequence.base_state())
  sequence.execute_frame()
print(time.time() - start_time)

start_time = time.time()
for _ in range(10):
  sequence.execute_frame()
print(time.time() - start_time)
