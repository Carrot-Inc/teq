// A class's variance is checked through a named tuple's values as through a plain tuple's: NamedTuple[N, +V] is
// covariant in its values, so a covariant parameter in a named tuple of a method's parameter stands in a
// contravariant position, and a contravariant one in a named tuple of a result in a covariant one.
trait Src[+T]:
  def put(x: (a: T, b: Int)): Unit
class Box[+T](val f: ((a: T, b: Int)) => Unit)
class Sink[-T](val v: (a: T, b: Int))
trait Res[-T]:
  def get: (a: T, b: Int)
abstract class Holder[+T]:
  def take(x: (a: T, b: Int)): Unit = ()
trait Fine[-T]:
  def put(x: (a: T, b: Int)): Unit
  def get: (a: Int, b: T => Unit)
// expect: 5:11: error: covariant type T occurs in contravariant position in type (a : T, b : Int) of parameter x
// expect: 6:19: error: covariant type T occurs in contravariant position in type (a : T, b : Int) => Unit of value f
// expect: 7:20: error: contravariant type T occurs in covariant position in type (a : T, b : Int) of value v
// expect: 9:7: error: contravariant type T occurs in covariant position in type => (a : T, b : Int) of method get
// expect: 11:12: error: covariant type T occurs in contravariant position in type (a : T, b : Int) of parameter x
