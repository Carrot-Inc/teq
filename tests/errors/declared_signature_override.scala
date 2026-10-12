// expect: 25:16: error: method alias overrides nothing
// expect: 26:16: error: method union overrides nothing
// expect: 28:7: error: class Impl needs to be abstract, since def t(x:
// expect: 38:16: error: method c overrides nothing
// expect: 41:7: error: class GChild needs to be abstract, since def f[
// expect: 42:16: error: method f overrides nothing
// expect: 45:7: error: class DeepChild needs to be abstract, since def f[
// expect: 46:16: error: method f overrides nothing
// A member overrides or implements another of its name only where their signatures match, which dotty takes
// from the erasure of the declarations (`Signature` by `sigName`, `Denotation.matches`): `x: Types.C` of a `*:` chain
// behind an alias is `Product`, `x: (Int, String)` `Tuple2`; `x: V | V` is `Object`, `x: V` the value class. (scalac
// words the first two "method alias has a different signature than the overridden declaration", E038, and shows
// the third's member as declared, `def t(x: Types.C): Types.C`.) A chain ending in `EmptyTuple.type` is the tuple class of
// its arity, one ending in the `EmptyTuple` alias `Product` (`tupleArity`): `s` overrides, `c` does not. A type
// parameter is named by its bound as declared (`sigName`): `T <: V | V` is `Object`, `T <: V` the value class, through
// a chain of bounds of any length (`DeepBase`'s ten).
class V(val x: Int) extends AnyVal
object Types:
  type C = Int *: String *: EmptyTuple
  type U = V | V
class Base:
  def alias(x: Types.C): Types.C = x
  def union(x: Types.U): Types.U = x
class Sub extends Base:
  override def alias(x: (Int, String)): (Int, String) = x
  override def union(x: V): V = x
trait Tr { def t(x: Types.C): Types.C }
class Impl extends Tr { def t(x: (Int, String)): (Int, String) = x }
@main def run() = println(new Sub)
object T2:
  type S = Int *: EmptyTuple.type
  type C1 = Int *: EmptyTuple
class B2:
  def s(x: T2.S): Int = 1
  def c(x: T2.C1): Int = 1
class D2 extends B2:
  override def s(x: Tuple1[Int]): Int = 2
  override def c(x: Tuple1[Int]): Int = 2
trait GBase:
  def f[T <: V | V](x: T): Int
class GChild extends GBase:
  override def f[T <: V](x: T): Int = 1
trait DeepBase:
  def f[A0 <: V | V, A1 <: A0, A2 <: A1, A3 <: A2, A4 <: A3, A5 <: A4, A6 <: A5, A7 <: A6, A8 <: A7, A9 <: A8](x: A9): Int
class DeepChild extends DeepBase:
  override def f[A0 <: V, A1 <: A0, A2 <: A1, A3 <: A2, A4 <: A3, A5 <: A4, A6 <: A5, A7 <: A6, A8 <: A7, A9 <: A8](x: A9): Int = 1
