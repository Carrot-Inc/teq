// The getter of a lazy val keeps the descriptor teq's erasure gives the member (the jvm-lazy-vals part changes none):
// a Unit (scala.runtime.BoxedUnit, TypeErasure.transformInfo), an Array[T], a generic value class over T and over
// Array[T], a tuple and a path-dependent result, beside the Object container every lazy member has.
// abi: unit unit$lzy1 arr arr$lzy1 wrapped wrapped$lzy1 wrappedArray wrappedArray$lzy1 pair pair$lzy1 inner inner$lzy1
package lazy_descriptors

class Wrap[T](val t: T) extends AnyVal
class WrapArray[T](val a: Array[T]) extends AnyVal

class Outer:
  class In

class Holder[T](make: () => Array[T]):
  lazy val unit: Unit = ()
  lazy val arr: Array[T] = make()
  lazy val wrapped: Wrap[String] = new Wrap("w")
  lazy val wrappedArray: WrapArray[Int] = new WrapArray(Array(1))
  lazy val pair: (Int, String) = (1, "one")
  val o = new Outer
  lazy val inner: o.In = new o.In
