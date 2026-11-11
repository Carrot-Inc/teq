// An alias applying a higher-kinded parameter (`type A[F[_]] = F[Int]`) to `List`, whose constructor teq holds as the
// eta-expansion `[X] =>> immutable.List[X]`: the JVM backend's erasure after typing expands the alias through
// `NoWorker` (src/types.rs), whose `applied_to` is the store's `appliedTo` (the `HKTypeLambda` arm instantiated,
// fac08057f1), so `C.m(x: A[List])` erases to `(Lscala/collection/immutable/List;)I` as scalac's `TypeErasure` gives
// it, implements `K.m` and the call through `K` prints 3. At 24df636e49 the lambda's application stayed unreduced
// (`Type::Applied`, erased to `Object`) and the call threw `AbstractMethodError` on the JVM. A type lambda's
// constructor, a super accessor and an opaque type over the shape erase alike. The closure adversary of the pass on
// 5bd85bcd1 (round 23); the round's base a5f69ed337 and master print 3.
type A[F[_]] = F[Int]
trait K:
  def m(x: List[Int]): Int
class C extends K:
  def m(x: A[List]): Int = x.length
@main def run(): Unit =
  val k: K = new C
  println(k.m(List(1, 2, 3)))
