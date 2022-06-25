import sys
import traceback
import platform

import wafel.log as log
import wafel.config as config

log.info("Wafel", config.version_str("."))
log.info(f"Platform: {platform.platform()} {platform.machine()}")

config.init()

with open(config.log_file, "a", encoding="utf-8") as log_file:
    log_file.write("-" * 80 + "\n")

    def append_to_log(message: log.LogMessage) -> None:
        log_file.write(str(message) + "\n")
        log_file.flush()

    log.subscribe(append_to_log)

    try:
        from wafel.main import run

        # import cProfile
        # cProfile.run('run()', sort='cumtime')
        run()
    except:
        log.error("Uncaught:", traceback.format_exc())
        sys.exit(1)
