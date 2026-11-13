// What an inline method's expansion reads as a constant, reader by reader, as scalac's readers do
// (`ConstantValue` for a condition and a guard, `ConstFold`'s for an operand, an `inline match`'s
// selector, a component of a construction that is the selector, a by-value parameter's binding):
// each line is one reader over one tree. A cast is read by its own type: no constant under a
// cast to `Boolean`, the constant under one to the singleton `true`.
object Lib:
  inline def yes: Boolean = true

object Gate:
  transparent inline def passV(m: Boolean): Boolean = m
  transparent inline def operand_null(inline n: Null): Int =
    inline if n == null then 100 else 200
  transparent inline def selector_null(inline n: Null): String =
    inline n match
      case null => "nine"
      case _ => "other"
  transparent inline def inlineIf_bare(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf_block(inline b: Boolean): Int =
    inline if { b } then 100 else 200
  transparent inline def inlineIf_call: Int =
    inline if Lib.yes then 100 else 200
  transparent inline def inlineIf_singleton(inline b: Boolean): Int =
    inline if b.asInstanceOf[true] then 100 else 200
  transparent inline def plainIf_bare(inline b: Boolean): Int =
    if b then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def plainIf_block(inline b: Boolean): Int =
    if { b } then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def plainIf_singleton(inline b: Boolean): Int =
    if b.asInstanceOf[true] then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def operand_bare(inline b: Boolean): Int =
    inline if !b then 200 else 100
  transparent inline def operand_call: Int =
    inline if !Lib.yes then 200 else 100
  transparent inline def operand_singleton(inline b: Boolean): Int =
    inline if !b.asInstanceOf[true] then 200 else 100
  transparent inline def selector_bare(inline b: Boolean): String =
    inline b match
      case true => "nine"
      case _ => "other"
  transparent inline def selector_block(inline b: Boolean): String =
    inline { b } match
      case true => "nine"
      case _ => "other"
  transparent inline def selector_call: String =
    inline Lib.yes match
      case true => "nine"
      case _ => "other"
  transparent inline def selector_cast(inline b: Boolean): String =
    inline b.asInstanceOf[Boolean] match
      case true => "nine"
      case _ => "other"
  transparent inline def selector_singleton(inline b: Boolean): String =
    inline b.asInstanceOf[true] match
      case true => "nine"
      case _ => "other"
  transparent inline def guard_bare(inline b: Boolean): String =
    inline 1 match
      case _ if b => "nine"
      case _ => "other"
  transparent inline def guard_block(inline b: Boolean): String =
    inline 1 match
      case _ if { b } => "nine"
      case _ => "other"
  transparent inline def guard_singleton(inline b: Boolean): String =
    inline 1 match
      case _ if b.asInstanceOf[true] => "nine"
      case _ => "other"
  transparent inline def component_bare(inline b: Boolean): String =
    inline (b, 3) match
      case (true, 3) => "nine"
      case _ => "other"
  transparent inline def component_cast(inline b: Boolean): String =
    inline (b.asInstanceOf[Boolean], 3) match
      case (true, 3) => "nine"
      case _ => "other"
  transparent inline def component_singleton(inline b: Boolean): String =
    inline (b.asInstanceOf[true], 3) match
      case (true, 3) => "nine"
      case _ => "other"
  transparent inline def byValue_bare(inline b: Boolean): Int =
    inline if passV(b) then 100 else 200

@main def run(): Unit =
  println("inlineIf, the literal: " + Gate.inlineIf_bare(true))
  println("inlineIf, the ascribed literal: " + Gate.inlineIf_bare((true: Boolean)))
  println("inlineIf, in an empty block: " + Gate.inlineIf_block((true: Boolean)))
  println("inlineIf, a plain inline call written there: " + Gate.inlineIf_call)
  println("inlineIf, under a cast to the singleton: " + Gate.inlineIf_singleton((true: Boolean)))
  println("plainIf, the literal: " + Gate.plainIf_bare(true))
  println("plainIf, the ascribed literal: " + Gate.plainIf_bare((true: Boolean)))
  println("plainIf, in an empty block: " + Gate.plainIf_block((true: Boolean)))
  println("plainIf, under a cast to the singleton: " + Gate.plainIf_singleton((true: Boolean)))
  println("operand, the literal: " + Gate.operand_bare(true))
  println("operand, the ascribed literal: " + Gate.operand_bare((true: Boolean)))
  println("operand, a plain inline call written there: " + Gate.operand_call)
  println("operand, under a cast to the singleton: " + Gate.operand_singleton((true: Boolean)))
  println("selector, the literal: " + Gate.selector_bare(true))
  println("selector, the ascribed literal: " + Gate.selector_bare((true: Boolean)))
  println("selector, in an empty block: " + Gate.selector_block((true: Boolean)))
  println("selector, a plain inline call written there: " + Gate.selector_call)
  println("selector, under a cast to Boolean: " + Gate.selector_cast((true: Boolean)))
  println("selector, under a cast to the singleton: " + Gate.selector_singleton((true: Boolean)))
  println("guard, the literal: " + Gate.guard_bare(true))
  println("guard, the ascribed literal: " + Gate.guard_bare((true: Boolean)))
  println("guard, in an empty block: " + Gate.guard_block((true: Boolean)))
  println("guard, under a cast to the singleton: " + Gate.guard_singleton((true: Boolean)))
  println("component, the literal: " + Gate.component_bare(true))
  println("component, under a cast to Boolean: " + Gate.component_cast((true: Boolean)))
  println("component, under a cast to the singleton: " + Gate.component_singleton((true: Boolean)))
  println("byValue, the literal: " + Gate.byValue_bare(true))
  println("operand, the null literal: " + Gate.operand_null(null))
  println("selector, the null literal: " + Gate.selector_null(null))
