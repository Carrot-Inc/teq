// An alias applying a higher-kinded parameter (`type A[F[_]] = F[Int]`) to `List`:
// `C.m(x: A[List])` erases to `(Lscala/collection/immutable/List;)I`,
// as scalac's `TypeErasure` gives it, implements `K.m`,
// and the call through `K` prints 3.
// A type lambda's constructor,
// a super accessor and
// an opaque type over the shape
// erase alike.
type A[F[_]] = F[Int]
trait K:
  def m(x: List[Int]): Int
class C extends K:
  def m(x: A[List]): Int = x.length
@main def run(): Unit =
  val k: K = new C
  println(k.m(List(1, 2, 3)))
