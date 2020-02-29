import sys

min_python_version = (3, 7)
if sys.version_info < min_python_version:
  sys.stderr.write('Python >=%d.%d is required\n' % min_python_version)
  sys.stderr.flush()
  sys.exit(1)

if sys.maxsize < 2**31 - 1 or sys.maxsize > 2**32:
  sys.stderr.write('32 bit Python is required\n')
  sys.stderr.flush()
  sys.exit(1)

from wafel.main import run

run()
