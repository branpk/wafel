from typing import *
from enum import Enum, auto
from datetime import datetime


class LogLevel(Enum):
  DEBUG = auto()
  INFO = auto()
  WARN = auto()
  ERROR = auto()

class LogMessage:
  def __init__(self, level: LogLevel, timestamp: datetime, message: str) -> None:
    self.level = level
    self.timestamp = timestamp
    self.message = message

  def __str__(self) -> str:
    timestamp = self.timestamp.isoformat(' ')
    return f'[{timestamp}] [{self.level.name}] {self.message}'


history: List[LogMessage] = []
subscribers: List[Callable[[LogMessage], None]] = []

def subscribe(callback: Callable[[LogMessage], None]) -> None:
  for message in history:
    callback(message)
  subscribers.append(callback)


def log(message: LogMessage) -> None:
  for callback in subscribers:
    callback(message)
  history.append(message)

def log_join(level: LogLevel, *words: object) -> None:
  message = ' '.join(map(str, words))
  log(LogMessage(level, datetime.now(), message))


def debug(*words: object) -> None:
  log_join(LogLevel.DEBUG, *words)

def info(*words: object) -> None:
  log_join(LogLevel.INFO, *words)

def warn(*words: object) -> None:
  log_join(LogLevel.WARN, *words)

def error(*words: object) -> None:
  log_join(LogLevel.ERROR, *words)


__all__ = ['LogLevel', 'LogMessage', 'debug', 'info', 'warn', 'error']
