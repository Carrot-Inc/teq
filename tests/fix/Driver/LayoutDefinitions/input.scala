/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Definitions as scalameta 4.17.0 lays them out (scalameta.txt): a def's type parameters and clauses in a
// `Member.ParamClauseGroup`, a class's in its `Type.ParamClause` and `Ctor.Primary` (empty, zero-width where the
// parser stands, where none is written), a clause's `implicit` or `using` its `mod` and each parameter's, `val` and
// `var` a parameter's `Mod.ValParam` and `Mod.VarParam`, `Type.Bounds`, `def this` a `Ctor.Secondary`, an enum case
// from its `case` and `case A, B` one `Defn.RepeatedEnumCase`, an extension's methods in a `Defn.ExtensionGroup`, a
// function type's parameters a `Type.FuncParamClause`, `A | B` and `? <: U` over their parts.
trait Show[A]:
  def show(a: A): String
class Plain
class Box[+A] private (val a: A, var n: Int)(implicit ev: Ordering[Int]):
  val marker = n // assert: DumpLayout.layout
  def this(a: A) = this(a, 0)(using Ordering.Int)
  def map[B](f: A => B)(using o: Ordering[Int]): Box[B] = new Box(f(a))
  def lower[T >: Int <: AnyVal](t: T): T = t
  def ctx[T: Ordering](t: T): T = t
  def hk[F[_]](f: F[Int]): Int = 1
  def nothing = 1
  def empty() = 2
  def anonymous(using Int)(x: Int): Int = x
trait Holder(x: Int)
enum Color(val rgb: Int):
  case Red extends Color(1)
  case Green extends Color(2)
enum Plain2:
  case A, B
  case C[T](t: T)
object Defs:
  given si: Show[Int] with
    def show(a: Int): String = ""
  given Show[String] = new Show[String] { def show(a: String) = a }
  given [T](using s: Show[T]): Show[List[T]] with
    def show(a: List[T]) = ""
  extension (x: Int)
    def twice: Int = x + x
    def plus(y: Int): Int = x + y
  extension [T](xs: List[T]) def second: T = xs(1)
  type F1[X] = List[X]
  type F2 >: Int <: AnyVal
  type F3
  type F4 = List[? <: AnyVal]
  type F5 = (Int, String) => Int
  type F6 = Int => Int
  type F7 = Int ?=> String
  type F8 = [X] =>> List[X]
  type F9 = [X] => X => X
