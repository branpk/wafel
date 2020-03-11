from enum import Enum, auto
import datetime


class LogLevel(Enum):
  DEBUG = auto()
  INFO = auto()
  WARN = auto()
  ERROR = auto()


def log(level: LogLevel, message: str) -> None:
  timestamp = datetime.datetime.now().isoformat(' ')
  print(f'[{timestamp}] [{level.name}] {message}')

def log_join(level: LogLevel, *words: object) -> None:
  message = ' '.join(map(str, words))
  log(level, message)


def debug(*words: object) -> None:
  log_join(LogLevel.DEBUG, *words)

def info(*words: object) -> None:
  log_join(LogLevel.INFO, *words)

def warn(*words: object) -> None:
  log_join(LogLevel.WARN, *words)

def error(*words: object) -> None:
  log_join(LogLevel.ERROR, *words)


__all__ = ['debug', 'info', 'warn', 'error']
