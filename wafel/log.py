from typing import *
from enum import Enum, auto
from datetime import datetime
import time


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


class Timer:
  WINDOW = 30

  def __init__(self) -> None:
    self.times: Dict[Tuple[str, ...], List[float]] = {}
    self.start_times: Dict[Tuple[str, ...], float] = {}
    self.stack: List[str] = []

  def begin(self, name: str) -> None:
    self.stack.append(name)
    path = tuple(self.stack)
    self.start_times[path] = time.time()

  def end(self) -> None:
    path = tuple(self.stack)
    self.stack.pop()
    start_time = self.start_times.pop(path)
    if path not in self.times:
      self.times[path] = []
    self.times[path].append((time.time() - start_time) * 1000)

  def end_begin(self, name: str) -> None:
    self.end()
    self.begin(name)

  def begin_frame(self) -> None:
    assert len(self.stack) == 0
    self.begin('frame')

  def end_frame(self) -> None:
    while len(self.stack) > 0:
      self.end()
    count = len(self.times[('frame',)])
    for times in self.times.values():
      while len(times) < count:
        times.append(0)
      while len(times) > Timer.WINDOW:
        times.pop(0)

  def get_times(self) -> Dict[Tuple[str, ...], float]:
    result = {}
    min_count = min(map(len, self.times.values()))
    for path, times in sorted(self.times.items()):
      times = times[:min_count]
      result[path] = sum(times) / len(times)
    return result


timer = Timer()


__all__ = [
  'LogLevel',
  'LogMessage',
  'debug',
  'info',
  'warn',
  'error',
  'timer',
]
