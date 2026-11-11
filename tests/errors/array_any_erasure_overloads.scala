trait A1:
  def f(x: Seq[Any]): Int
  def f(x: Array[Any]): Int = 1
trait A2:
  def f(x: Any): Int
  def f(x: Array[Any]): Int = 1
trait A3:
  def f(x: Array[?]): Int
  def f(x: Array[Any]): Int = 1
trait A4:
  def f(x: Any): Int
  def f(x: Array[?]): Int = 1
trait A5[T]:
  def f(x: Array[T]): Int
  def f(x: Any): Int = 1
trait A6:
  def f(x: Array[AnyRef]): Int
  def f(x: Array[Any]): Int = 1
// The erased parameter of each pair, as scalac 3.8.4 gives it: Seq and Object[] (A1), Object and Object[] (A2),
// Object[] and Object (A3) differ; Object and Object (A4: `Array[?]` is a generic array), Object and Object (A5:
// `Array[T]` of an unbounded T), Object[] and Object[] (A6: `Any` is a class, so `Array[Any]` is no generic array).
// expect: 12:7: error: Conflicting definitions:
// expect: def f(x: Any): Int in trait A4 at line 11 and
// expect: def f(x: Array[?]): Int in trait A4 at line 12
// expect: 15:7: error: Conflicting definitions:
// expect: def f(x: Array[T]): Int in trait A5 at line 14 and
// expect: def f(x: Any): Int in trait A5 at line 15
// expect: 18:7: error: Conflicting definitions:
// expect: def f(x: Array[AnyRef]): Int in trait A6 at line 17 and
// expect: def f(x: Array[Any]): Int in trait A6 at line 18
// expect: 3 errors found
