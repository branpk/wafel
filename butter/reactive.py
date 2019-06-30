from typing import Generic, TypeVar, Callable, Tuple, Any, List


T = TypeVar('T')
S = TypeVar('S')


class Reactive(Generic[T]):
  """A Reactive[T] is conceptually a time-varying T value."""

  @staticmethod
  def tuple(*args: 'Reactive[Any]') -> 'Reactive[Tuple[Any, ...]]':
    """Turn a Tuple[Reactive[T1], Reactive[T2], ...] into a Reactive[Tuple[T1, T2, ...]]."""
    return _ReactiveTuple(tuple(args))

  @property
  def value(self) -> T:
    """Get the value at the current point in time."""
    raise NotImplementedError

  def on_change(self, callback: Callable[[T], None]) -> None:
    """Register a callback to be called whenever this value changes.

    It is not guaranteed that .value will actually change when this happens. It
    depends on the type of Reactive.
    """
    raise NotImplementedError

  def map(self, func: Callable[[T], S]) -> 'Reactive[S]':
    """Map a function onto a Reactive.

    This does not "narrow" the value. E.g. if the source value is (1, 2, 3),
    the function is lambda p: p[0], and the source value changes to (1, 4, 3),
    the final value will call its on_change callbacks.
    """
    return _ReactiveMap(self, func)

  def mapn(self: 'Reactive[tuple]', func: Callable[..., S]) -> 'Reactive[S]':
    """Treat the tuple as containing the arguments to the map function.

    This allows Reactive.tuple(a, b).mapn(lambda x, y: x + y) for example.
    """
    return self.map(lambda args: func(*args))


class ReactiveValue(Reactive[T]):
  """A variable that can be get/set.

  The value only calls its on_change callbacks when the value actually changes,
  so best practice is to only use immutable values.
  """

  def __init__(self, value: T) -> None:
    self._value = value
    self._callbacks: List[Callable[[T], None]] = []

  @property
  def value(self) -> T:
    return self._value

  @value.setter
  def value(self, value: T) -> None:
    if self._value != value:
      self._value = value
      for callback in list(self._callbacks):
        callback(self._value)

  def on_change(self, callback: Callable[[T], None]) -> None:
    self._callbacks.append(callback)


class _ReactiveMap(Reactive[S]):
  def __init__(self, source: Reactive[T], func: Callable[[T], S]) -> None:
    self._source = source
    self._func = func

  @property
  def value(self) -> S:
    return self._func(self._source.value)

  def on_change(self, callback: Callable[[S], None]) -> None:
    def source_callback(source: T) -> None:
      callback(self._func(source))
    self._source.on_change(source_callback)


class _ReactiveTuple(Reactive[Tuple[Any, ...]]):
  def __init__(self, sources: Tuple[Reactive[Any], ...]) -> None:
    self._sources = sources

  @property
  def value(self) -> Tuple[Any, ...]:
    return tuple(s.value for s in self._sources)

  def on_change(self, callback: Callable[[Tuple[Any, ...]], None]) -> None:
    def make_source_callback(index: int) -> Callable[[Any], None]:
      def source_callback(source: Any) -> None:
        prefix = tuple(s.value for s in self._sources[:index])
        suffix = tuple(s.value for s in self._sources[index+1:])
        callback(prefix + (source,) + suffix)
      return source_callback

    for i, source in enumerate(self._sources):
      source.on_change(make_source_callback(i))
