// The cells of tests/cases/inline_constant_readers.scala that scalac's typer refuses, each alone:
// a cast to `Boolean` is read by its own type, no constant, for a condition, an operand, a guard
// and a by-value parameter (whose binding takes no cast's type, a singleton's either); a cast to
// `Boolean` over a cast to the singleton is the outer cast, whatever stands under both. A plain
// `if` that is not reduced keeps its other branch for the inlining phase, which reports it
// where the typer reported nothing (tests/errors/inline_constant_kept_branch.scala).
// expect: 34:13: error: Cannot reduce `inline if` because its condition is not a constant value: b.asInstanceOf[Boolean]
// expect: 35:13: error: Cannot reduce `inline if` because its condition is not a constant value: !b.asInstanceOf[Boolean]
// expect: 36:13: error: cannot reduce inline match with
// expect: 37:13: error: Cannot reduce `inline if` because its condition is not a constant value: passV(b.asInstanceOf[Boolean])
// expect: 38:13: error: Cannot reduce `inline if` because its condition is not a constant value: passV(b.asInstanceOf[true])
// expect: 39:13: error: Cannot reduce `inline if` because its condition is not a constant value: b.asInstanceOf[true].asInstanceOf[Boolean]
object Lib:
  def answer(): Boolean = false
  inline def yes: Boolean = true

object Gate:
  transparent inline def passV(m: Boolean): Boolean = m
  transparent inline def inlineIf_twoCasts(inline b: Boolean): Int =
    inline if b.asInstanceOf[true].asInstanceOf[Boolean] then 100 else 200
  transparent inline def inlineIf_cast(inline b: Boolean): Int =
    inline if b.asInstanceOf[Boolean] then 100 else 200
  transparent inline def operand_cast(inline b: Boolean): Int =
    inline if !b.asInstanceOf[Boolean] then 200 else 100
  transparent inline def guard_cast(inline b: Boolean): String =
    inline 1 match
      case _ if b.asInstanceOf[Boolean] => "nine"
      case _ => "other"
  transparent inline def byValue_cast(inline b: Boolean): Int =
    inline if passV(b.asInstanceOf[Boolean]) then 100 else 200
  transparent inline def byValue_singleton(inline b: Boolean): Int =
    inline if passV(b.asInstanceOf[true]) then 100 else 200

val cell0 = Gate.inlineIf_cast((true: Boolean)) // inlineIf, under a cast to Boolean
val cell1 = Gate.operand_cast((true: Boolean)) // operand, under a cast to Boolean
val cell2 = Gate.guard_cast((true: Boolean)) // guard, under a cast to Boolean
val cell3 = Gate.byValue_cast((true: Boolean)) // byValue, under a cast to Boolean
val cell4 = Gate.byValue_singleton((true: Boolean)) // byValue, under a cast to the singleton
val cell5 = Gate.inlineIf_twoCasts(Lib.answer()) // inlineIf, a cast to Boolean over a cast to the singleton
