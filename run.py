import sys
import os
import traceback

import wafel.log as log
from wafel.config import config, VERSION
from wafel.main import run

try:
  log.info('Wafel', '.'.join(map(str, VERSION)))
  log.subscribe(print)

  if getattr(sys, 'frozen', False):
    root_dir = os.path.dirname(sys.executable)
    config._lib_directory = root_dir
  else:
    root_dir = os.path.dirname(os.path.abspath(__file__))
    config._lib_directory = os.path.join(root_dir, 'lib')

  config._assets_directory = os.path.join(root_dir, 'assets')
  config._cache_directory = os.path.join(root_dir, '.wafel_cache')
  config._log_file = os.path.join(root_dir, 'log.txt')

  with open(config.log_file, 'w') as log_file:
    def append_to_log(message: log.LogMessage) -> None:
      log_file.write(str(message) + '\n')
      log_file.flush()
    log.subscribe(append_to_log)

    run()

except:
  log.error('Uncaught:', traceback.format_exc())
  sys.exit(1)
