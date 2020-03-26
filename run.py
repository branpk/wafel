import sys
import os
import traceback

import wafel.log as log
import wafel.config as config

log.info('Wafel', config.version_str('.'))
log.subscribe(print)

if getattr(sys, 'frozen', False):
  config.dev_mode = False
  root_dir = os.path.dirname(sys.executable)
  config.lib_directory = root_dir
else:
  config.dev_mode = '--nodev' not in sys.argv
  root_dir = os.path.dirname(os.path.abspath(__file__))
  config.lib_directory = os.path.join(root_dir, 'lib')

config.assets_directory = os.path.join(root_dir, 'assets')
config.cache_directory = os.path.join(root_dir, '.wafel_cache')
config.log_file = os.path.join(root_dir, 'log.txt')
config.settings_file = os.path.join(root_dir, 'settings.json')

with open(config.log_file, 'a') as log_file:
  def append_to_log(message: log.LogMessage) -> None:
    log_file.write(str(message) + '\n')
    log_file.flush()
  log.subscribe(append_to_log)

  try:
    from wafel.main import run
    # import cProfile
    # cProfile.run('run()', sort='tottime')
    run()
  except:
    log.error('Uncaught:', traceback.format_exc())
    sys.exit(1)
