// A cast to a path's singleton type is read by what the path stands for (scalac widens the
// path: `widenTermRefExpr`): the constant of a `final val`, of a val typed with a literal type,
// of an `inline val`, of a local val; none for a val of the type `Int`. The argument's own value
// does not decide: 8 under a cast to the type of a path that stands for 9 reads as 9.
object Paths:
  final val fixed = 9
  val typed: 9 = 9
  val wide: Int = 9
  inline val inlined = 9
  final val word = "a"
  final val yes = true
  object Inner:
    final val deep = 9

object Gate:
  transparent inline def finalVal(inline n: Int): String =
    inline n.asInstanceOf[Paths.fixed.type] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def typedVal(inline n: Int): String =
    inline n.asInstanceOf[Paths.typed.type] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def wideVal(inline n: Int): String =
    inline n.asInstanceOf[Paths.wide.type] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def inlineVal(inline n: Int): String =
    inline n.asInstanceOf[Paths.inlined.type] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def nestedVal(inline n: Int): String =
    inline n.asInstanceOf[Paths.Inner.deep.type] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def localVal(inline n: Int): String =
    val here: 9 = 9
    inline n.asInstanceOf[here.type] match
      case 9 => "nine"
      case _ => "other"
  inline def inThePhase(inline n: Int): String =
    inline n.asInstanceOf[Paths.fixed.type] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def text(inline s: String): String =
    inline s.asInstanceOf[Paths.word.type] match
      case "a" => "nine"
      case _ => "other"
  transparent inline def condition(inline b: Boolean): Int =
    inline if b.asInstanceOf[Paths.yes.type] then 100 else 200
  transparent inline def plainCondition(inline b: Boolean): Int =
    if b.asInstanceOf[Paths.yes.type] then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def operand(inline n: Int): Int =
    inline if n.asInstanceOf[Paths.fixed.type] + 1 == 10 then 100 else 200
  transparent inline def guard(inline b: Boolean): String =
    inline 1 match
      case _ if b.asInstanceOf[Paths.yes.type] => "nine"
      case _ => "other"
  transparent inline def component(inline n: Int): String =
    inline (n.asInstanceOf[Paths.fixed.type], 3) match
      case (9, 3) => "nine"
      case _ => "other"

@main def run(): Unit =
  println("a final val's path: " + Gate.finalVal((9: Int)))
  println("a final val's path, another value under it: " + Gate.finalVal((8: Int)))
  println("a final val's path, a literal under it: " + Gate.finalVal(9))
  println("a val typed with a literal type: " + Gate.typedVal((9: Int)))
  println("a val of the type Int: " + Gate.wideVal((9: Int)))
  println("an inline val: " + Gate.inlineVal((9: Int)))
  println("a val of an inner object: " + Gate.nestedVal((9: Int)))
  println("a local val: " + Gate.localVal((9: Int)))
  println("in a method the inlining phase expands: " + Gate.inThePhase((9: Int)))
  println("a String's path: " + Gate.text(("a": String)))
  println("a condition: " + Gate.condition((true: Boolean)))
  println("a condition, another value under it: " + Gate.condition((false: Boolean)))
  println("a plain condition: " + Gate.plainCondition((true: Boolean)))
  println("an operand: " + Gate.operand((9: Int)))
  println("a guard: " + Gate.guard((true: Boolean)))
  println("a component: " + Gate.component((9: Int)))
