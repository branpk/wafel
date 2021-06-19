from typing import *
from dataclasses import dataclass
import time
import sys


T = TypeVar('T')

@dataclass(frozen=True)
class Progress:
  progress: float
  status: str

Loading = Generator[Progress, None, T]

def in_progress(progress: float, status: str = '') -> Progress:
  return Progress(progress, status)

def load_scale(start: float, stop: float, child: Loading[T]) -> Loading[T]:
  try:
    while True:
      progress = next(child)
      yield in_progress(
        start + progress.progress * (stop - start),
        progress.status,
      )
  except StopIteration as result:
    return cast(T, result.value)

def load_prefix(prefix: str, child: Loading[T]) -> Loading[T]:
  try:
    while True:
      progress = next(child)
      yield in_progress(
        progress.progress,
        prefix + progress.status,
      )
  except StopIteration as result:
    return cast(T, result.value)

def load_child(start: float, stop: float, prefix: str, child: Loading[T]) -> Loading[T]:
  return load_prefix(prefix, load_scale(start, stop, child))


def finish_loading(loader: Loading[T]) -> T:
  try:
    while True:
      next(loader)
  except StopIteration as result:
    return cast(T, result.value)


def test_loading_timing(loader: Loading[object]) -> None:
  start_time = time.time()
  progresses = []
  for progress in loader:
    progresses.append((progress, time.time()))
  end_time = time.time()

  for progress, timestamp in progresses:
    print(progress.progress, (timestamp - start_time) / (end_time - start_time), progress.status)

  timestamps = [start_time] + [t for _, t in progresses] + [end_time]
  print('max wait =', max(t2 - t1 for t1, t2 in zip(timestamps, timestamps[1:])))

def test_loading(loader: Loading[object]) -> None:
  prev_length = 0
  for progress in loader:
    xs = int(progress.progress * 100)
    line = '[' + 'X' * xs + ' ' * (100 - xs) + '] ' + progress.status
    sys.stdout.write('\r' + ' ' * prev_length + '\r' + line)
    sys.stdout.flush()
    prev_length = len(line)
  print()
