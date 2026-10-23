/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Shapes scalameta 4.17.4 gives a compiling program under the Scala 3 dialect: a `_` type argument inside two type
// brackets (an argument's or a parameter clause's) is a placeholder of a `Type.AnonymousLambda`, in one bracket, in a
// pattern's type or bounded a `Type.Wildcard`; a symbolic infix type is a `Type.ApplyInfix` to its right operand's
// own end; an empty case body, an `if` without `else` and a missing `return` value stand at scalameta's current
// token; indented cases a comma closes end after it.
object Placeholders:
  val marker = 0 // assert: DumpLayout.layout
  type One = List[_]
  type Two = Map[String, List[_]]
  type Bounded = Map[String, List[_ <: AnyVal]]
  type Infix = Int *: (String *: EmptyTuple)
  def param(m: Map[String, List[_]]): Int = m.size
  val applied = List.empty[Option[_]]
  class Bound[A <: List[_]]
  def pattern(x: Any): Int = x match
    case _: Map[_, List[_]] => 1
    case _ => 0
  def braced(x: Int): Unit = x match {
    case 1 =>
    case _ =>
  }
  def indented(x: Int): Unit =
    x match
      case 1 =>
      case _ =>

  def noElse(c: Boolean): Unit =
    if c then
      println(1)
  def early(c: Boolean): Unit =
    if c then return
    println(2)
  def pair(a: Int, b: Int): Int = a + b
  val closed = pair(
    1 match
      case 1 => 2
      case _ => 3,
    4,
  )
