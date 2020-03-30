from __future__ import annotations

from typing import *
from enum import Enum, auto
from datetime import datetime
import time
from dataclasses import dataclass
import math


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
subscribers: List[Callable[[LogMessage], None]] = [print]

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



@dataclass
class Summary:
  time: float = 0.0
  copies: float = 0.0
  updates: float = 0.0
  requests: float = 0.0

  @staticmethod
  def average(samples: List[Summary]) -> Summary:
    if len(samples) == 0:
      return Summary(0)
    return Summary(
      time = sum(s.time for s in samples) / len(samples),
      copies = sum(s.copies for s in samples) / len(samples),
      updates = sum(s.updates for s in samples) / len(samples),
      requests = sum(s.requests for s in samples) / len(samples),
    )


class Timer:
  WINDOW = 30

  def __init__(self) -> None:
    self.samples: Dict[Tuple[str, ...], List[Summary]] = {}
    self.active: Dict[Tuple[str, ...], Summary] = {}
    self.stack: List[str] = []

  def begin(self, name: str) -> None:
    self.stack.append(name)
    path = tuple(self.stack)
    self.active[path] = Summary(time=time.time())

  def end(self) -> None:
    path = tuple(self.stack)
    self.stack.pop()
    if path not in self.samples:
      self.samples[path] = []
    sample = self.active.pop(path)
    sample.time = (time.time() - sample.time) * 1000
    self.samples[path].append(sample)

  def begin_frame(self) -> None:
    assert len(self.stack) == 0
    self.begin('frame')

  def end_frame(self) -> None:
    while len(self.stack) > 0:
      self.end()
    count = len(self.samples[('frame',)])
    for samples in self.samples.values():
      while len(samples) < count:
        samples.append(Summary())
      while len(samples) > Timer.WINDOW:
        samples.pop(0)

  def record_copy(self) -> None:
    for sample in self.active.values():
      sample.copies += 1

  def record_update(self) -> None:
    for sample in self.active.values():
      sample.updates += 1

  def record_request(self) -> None:
    for sample in self.active.values():
      sample.requests += 1

  def get_summaries(self) -> Dict[Tuple[str, ...], Summary]:
    result = {}
    min_count = min(map(len, self.samples.values()))
    for path, samples in sorted(self.samples.items()):
      samples = samples[:min_count]
      result[path] = Summary.average(samples)
    return result

  def get_frame_summary(self) -> Dict[Tuple[str, ...], Summary]:
    result = {}
    min_count = min(map(len, self.samples.values()))
    if min_count == 0:
      return {}
    for path, samples in sorted(self.samples.items()):
      samples = samples[:min_count]
      result[path] = samples[-1]
    return result

  def format(self, summaries: Dict[Tuple[str, ...], Summary]) -> List[str]:
    from wafel.util import format_align
    return format_align(
      '{0}%s%a - %s{1:.1f}%ams  %s{2}%a  %s{3}%a  %s{4}%a',
      [
        (
          '  ' * (len(path) - 1) + path[-1],
          s.time,
          math.ceil(s.copies),
          math.ceil(s.updates),
          math.ceil(s.requests),
        )
        for path, s in summaries.items()
      ],
    )


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
