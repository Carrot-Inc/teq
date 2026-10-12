package ea

// The shapes whose erasure scalac reads from the declaration (`TypeErasure`): a union of a value class with itself
// (`OrType`'s `erasedLub` of two `ErasedValueType`s, `Object`), a `*:` chain behind an alias, a second alias and a
// generic alias (`tupleArity` of the `EmptyTuple` alias, `Product`), and an inferred `Nothing` or `Null` result
// (`CoreBTypesFromSymbols`, `Nothing$` and `Null$`); the review's shapes (`Reviewed`): a union a substitution makes of
// one type twice (`derivedOrType`, the underlying `int`), `Null | Null` and `Nothing | Nothing` (the markers), the views
// through an intersection, a refinement and a by-name parameter, an abstract member's bound and a path to it; the
// second review's (`Bottoms`): a bottom kept through an intersection, a refinement and an abstract member's bound.
class V(val x: Int) extends AnyVal

object Types:
  type C = Int *: String *: EmptyTuple
  type D = C
  type G[A] = A *: EmptyTuple
  type U = V | V
  type U2[A] = A | A
  type B[A <: V] = A | A
  type BV = Null | Null

class API:
  def union[T <: V | V](x: T): T = x
  def param(x: V | V): V | V = x
  def aliasU(x: Types.U): Types.U = x
  def strs(x: String | String): String | String = x
  def alias(x: Types.C): Types.C = x
  def second(x: Types.D): Types.D = x
  def generic(x: Types.G[Int]): Types.G[Int] = x
  def arr(x: Array[Types.C]): Array[Types.C] = x
  def tbound[T <: Types.C](x: T): T = x
  def charBound[T <: Char](x: T): T = x
  def inferredNull = null
  def inferredNothing(fail: Boolean) = if fail then throw new IllegalStateException("thrown") else ???

class Holder(val chain: Types.C)

trait AbsAPI:
  type T <: V | V
  def f(x: T): T

class Abs extends AbsAPI:
  type T = V
  def f(x: V): V = new V(x.x + 1)

class Reviewed:
  def applied(x: Types.U2[V]): Types.U2[V] = x
  def bounded(x: Types.B[V]): Types.B[V] = x
  def inter(x: (V | V) & Any): (V | V) & Any = x
  def chainInter(x: Types.G[Int] & Product): Types.G[Int] & Product = x
  def refine(x: Types.G[Int] { type T = Int }): Types.G[Int] { type T = Int } = x
  def byName(x: => V | V): V | V = x
  def n: Null | Null = null
  def p(x: Null | Null): Null | Null = x
  def nb[T <: Null | Null](x: T): T = x
  val v: Null | Null = null
  def dep(a: AbsAPI)(x: a.T): a.T = a.f(x)

trait BottomAPI:
  type T <: Null | Null
  def f(x: T): T

class BottomImpl extends BottomAPI:
  type T = Null
  def f(x: Null): Null = x

class Bottoms:
  def bottomInter(x: (Null | Null) & Any): (Null | Null) & Any = x
  def bottomRefine(x: Types.BV { type T = Int }): Types.BV { type T = Int } = x
  def bottomDep(a: BottomAPI)(x: a.T): a.T = a.f(x)
