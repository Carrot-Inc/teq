// Erasure as dotty's `TypeErasure` has it: a `*:` chain as an upper bound erases as the chain spelled out does, to
// `Product` (master: the tuple class of its elements); a union of `String`
// and a class to their erased lub, `CharSequence` of `String | java.lang.StringBuilder`, `Serializable` of
// `String | List[Int]` (master: `Object`), by the first operand's linearisation, `Both | String` to `Serializable`
// where `String | Both` is `CharSequence`, an array of either the array of the lub; an applied opaque type to its
// underlying type over its arguments, `Arr[Int]` of `opaque type Arr[A] = Array[A]` to `[I`, where a generic one
// stays `Object`. `Nothing` and `Null`, written or inferred, erase to `Nothing$` and `Null$` in a signature
// (`CoreBTypesFromSymbols`; master: `Object` where inferred). `nullField(` lists the getter alone: teq keeps a
// private field for a val of type `Null`, `Nothing` or `Unit` where scalac's `Memoize` keeps none
// (`isErasableBottomField`), a difference left. `Declared` holds what the erasure reads of the declaration, which
// the type normalizes away (the erasure provenance E1; master: the type's erasure): a union of a value class with
// itself (`erasedLub` of two `ErasedValueType`s, `Object`), a `*:` chain behind an alias, a second alias and a generic
// alias (`tupleArity` of the `EmptyTuple` alias, `Product`), as a parameter, a result, a bound, an array's element and
// a class parameter. `Reviewed` holds the review's shapes (2026-10-12): a union an alias's substitution makes of one
// type twice, which dotty's `derivedOrType` simplifies (`U2[V]`, `B[V]`: the underlying `int`); a union of `Null` or
// `Nothing` with itself, whose erased bottom is the runtime's marker; the views through an intersection, a refinement
// and a by-name parameter; an abstract member's bound and a path to it. `Bottoms` and `BottomAPI` hold the second
// review's: a bottom kept through the whole erasure, an intersection's, a refinement's over an alias of `Null | Null`
// and an abstract member's bound's, to the runtime's marker; and its controls.
// abi: API#bound API#direct API#union API#serial API#rserial API#left API#right API#arrays API#nullable API#prim API#spec API#gen API#inferredNull API#inferredNothing API#nullField( API#writtenNull Declared#<init> Declared#chain Declared#union Declared#param Declared#aliasU Declared#strs Declared#alias Declared#second Declared#generic Declared#single Declared#arr Declared#tbound Declared#inter Declared#nothingU Reviewed#applied Reviewed#bounded Reviewed#inter2 Reviewed#chainInter Reviewed#refine Reviewed#byName Reviewed#n Reviewed#z Reviewed#p Reviewed#nb Reviewed#v( Reviewed#dep AbsAPI#f Bottoms#bottomInter Bottoms#bottomRefine Bottoms#pz Bottoms#nz Bottoms#bottomClass Bottoms#nested Bottoms#nestedParam Bottoms#distinct Bottoms#nullable Bottoms#interVC Bottoms#refineVC BottomAPI#f
package erasure
opaque type Arr[A] = Array[A]
object Arr { def ints(n: Int): Arr[Int] = new Array[Int](n) }
class Both extends CharSequence with java.io.Serializable:
  def length: Int = 0
  def charAt(i: Int): Char = 'a'
  def subSequence(a: Int, b: Int): CharSequence = ""
class API:
  def bound[T <: Int *: EmptyTuple](x: T): T = x
  def direct(x: Int *: String *: EmptyTuple): Int *: String *: EmptyTuple = x
  def union(x: String | java.lang.StringBuilder): String | java.lang.StringBuilder = x
  def serial(x: String | List[Int]): String | List[Int] = x
  def rserial(x: List[Int] | String): List[Int] | String = x
  def left(x: String | Both): String | Both = x
  def right(x: Both | String): Both | String = x
  def arrays(x: Array[String] | Array[Both]): Array[String] | Array[Both] = x
  def nullable(x: String | Null): String | Null = x
  def prim(x: String | Int): String | Int = x
  def spec(x: Arr[Int]): Arr[Int] = x
  def gen[A](x: Arr[A]): Arr[A] = x
  def inferredNull = null
  def inferredNothing = throw new Exception("never")
  val nullField = null
  def writtenNull(x: Null): Null = x
class V(val x: Int) extends AnyVal
object Types:
  type C = Int *: String *: EmptyTuple
  type D = C
  type G[A] = A *: EmptyTuple
  type U = V | V
  type S = Int *: EmptyTuple.type
  type U2[A] = A | A
  type B[A <: V] = A | A
  type N[A] = U2[A]
  type N2[A] = N[A]
  type UV = V | V
  type BV = Null | Null
class Declared(val chain: Types.C):
  def union[T <: V | V](x: T): T = x
  def param(x: V | V): V | V = x
  def aliasU(x: Types.U): Types.U = x
  def strs(x: String | String): String | String = x
  def alias(x: Types.C): Types.C = x
  def second(x: Types.D): Types.D = x
  def generic(x: Types.G[Int]): Types.G[Int] = x
  def single(x: Types.S): Types.S = x
  def arr(x: Array[Types.C]): Array[Types.C] = x
  def tbound[T <: Types.C](x: T): T = x
  def inter(x: V & V): V & V = x
  def nothingU(x: V | Nothing): V | Nothing = x
trait AbsAPI:
  type T <: V | V
  def f(x: T): T
class Reviewed:
  def applied(x: Types.U2[V]): Types.U2[V] = x
  def bounded(x: Types.B[V]): Types.B[V] = x
  def inter2(x: (V | V) & Any): (V | V) & Any = x
  def chainInter(x: Types.G[Int] & Product): Types.G[Int] & Product = x
  def refine(x: Types.G[Int] { type T = Int }): Types.G[Int] { type T = Int } = x
  def byName(x: => V | V): V | V = x
  def n: Null | Null = null
  def z: Nothing | Nothing = throw new Exception("never")
  def p(x: Null | Null): Null | Null = x
  def nb[T <: Null | Null](x: T): T = x
  val v: Null | Null = null
  def dep(a: AbsAPI)(x: a.T): a.T = x
class W(val x: Long) extends AnyVal
trait BottomAPI:
  type T <: Null | Null
  def f(x: T): T
class Bottoms:
  def bottomInter(x: (Null | Null) & Any): (Null | Null) & Any = x
  def bottomRefine(x: Types.BV { type T = Int }): Types.BV { type T = Int } = x
  def pz(x: Nothing | Nothing): Nothing | Nothing = x
  def nz[T <: Nothing | Nothing](x: T): T = x
  def bottomClass(x: Nothing | String): Nothing | String = x
  def nested(x: Types.N2[V]): Types.N2[V] = x
  def nestedParam[T <: V](x: Types.N2[T]): Types.N2[T] = x
  def distinct(x: V | W): V | W = x
  def nullable(x: V | Null): V | Null = x
  def interVC(x: V & W): V & W = x
  def refineVC(x: Types.UV { type T = Int }): Types.UV { type T = Int } = x
