// The cells of tests/cases/inline_parameter_cast_argument.scala that scalac's typer refuses, each
// with a reader of its own (scalac hides a second message at one inlined position): a cast to
// `Boolean` on an `inline` parameter's argument is no constant for a condition, an operand and a
// guard, whatever stands under it and through a block or parentheses; a by-value parameter of the
// reader takes no constant from a cast, to a singleton either; and a parameter that is not
// `inline`, by name or by value, is a proxy of the widened type for a cast and for a call of a
// singleton result type.
// expect: 119:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 120:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 121:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 122:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 123:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 124:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 125:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 126:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 127:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 128:13: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 129:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 130:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 131:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 132:14: error: cannot reduce inline match with
// expect: 133:14: error: cannot reduce inline match with
// expect: 134:14: error: cannot reduce inline match with
// expect: 135:14: error: cannot reduce inline match with
// expect: 136:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 137:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 138:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 139:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 140:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 141:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 142:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 143:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 144:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 145:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 146:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 147:14: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 29 errors found
object Source:
  def on(): Boolean = true
  def off(): Boolean = false
  def count(): Int = 8
object Pins:
  final val t = true
  final val nine = 9
  type Wide = Boolean
  def held(): t.type = { println("held"); t }
  def quiet(): t.type = t
object Tap:
  def on(): Boolean = { println("evaluated"); true }
object Dial:
  transparent inline def passV(m: Boolean): Boolean = m
  transparent inline def inlineIf0(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf1(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf2(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf3(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf4(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf5(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf6(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf7(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def inlineIf8(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def operand9(inline b: Boolean): Int =
    inline if !b then 200 else 100
  transparent inline def operand10(inline b: Boolean): Int =
    inline if !b then 200 else 100
  transparent inline def operand11(inline b: Boolean): Int =
    inline if !b then 200 else 100
  transparent inline def operand12(inline b: Boolean): Int =
    inline if !b then 200 else 100
  transparent inline def guard13(inline b: Boolean): String =
    inline 1 match
      case _ if b => "nine"
      case _ => "other"
  transparent inline def guard14(inline b: Boolean): String =
    inline 1 match
      case _ if b => "nine"
      case _ => "other"
  transparent inline def guard15(inline b: Boolean): String =
    inline 1 match
      case _ if b => "nine"
      case _ => "other"
  transparent inline def guard16(inline b: Boolean): String =
    inline 1 match
      case _ if b => "nine"
      case _ => "other"
  transparent inline def byValue17(inline b: Boolean): Int =
    inline if passV(b) then 100 else 200
  transparent inline def byValue18(inline b: Boolean): Int =
    inline if passV(b) then 100 else 200
  transparent inline def byValue19(inline b: Boolean): Int =
    inline if passV(b) then 100 else 200
  transparent inline def byValue20(inline b: Boolean): Int =
    inline if passV(b) then 100 else 200
  transparent inline def byValue21(inline b: Boolean): Int =
    inline if passV(b) then 100 else 200
  transparent inline def byValue22(inline b: Boolean): Int =
    inline if passV(b) then 100 else 200
  transparent inline def byName23(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def byName24(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def byName25(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def plain26(b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def plain27(b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def plain28(b: Boolean): Int =
    inline if b then 100 else 200

val cell0 = Dial.inlineIf0(true.asInstanceOf[Boolean]) // inlineIf, a cast to Boolean over the literal
val cell1 = Dial.inlineIf1((true: Boolean).asInstanceOf[Boolean]) // inlineIf, a cast to Boolean over an ascribed literal
val cell2 = Dial.inlineIf2(Source.on().asInstanceOf[Boolean]) // inlineIf, a cast to Boolean over a call
val cell3 = Dial.inlineIf3(Pins.t.asInstanceOf[Boolean]) // inlineIf, a cast to Boolean over a final val
val cell4 = Dial.inlineIf4((true: Boolean).asInstanceOf[Pins.Wide]) // inlineIf, a cast to an alias of Boolean
val cell5 = Dial.inlineIf5(false.asInstanceOf[true].asInstanceOf[Boolean]) // inlineIf, a cast to Boolean over a cast to the singleton
val cell6 = Dial.inlineIf6({ true.asInstanceOf[Boolean] }) // inlineIf, a block of a cast to Boolean
val cell7 = Dial.inlineIf7({ val u = 0; true.asInstanceOf[Boolean] }) // inlineIf, a block with a statement ending in a cast to Boolean
val cell8 = Dial.inlineIf8((true.asInstanceOf[Boolean])) // inlineIf, a parenthesised cast to Boolean
val cell9 = Dial.operand9(true.asInstanceOf[Boolean]) // operand, a cast to Boolean over the literal
val cell10 = Dial.operand10(Source.on().asInstanceOf[Boolean]) // operand, a cast to Boolean over a call
val cell11 = Dial.operand11(false.asInstanceOf[true].asInstanceOf[Boolean]) // operand, a cast to Boolean over a cast to the singleton
val cell12 = Dial.operand12({ val u = 0; true.asInstanceOf[Boolean] }) // operand, a block with a statement ending in a cast to Boolean
val cell13 = Dial.guard13(true.asInstanceOf[Boolean]) // guard, a cast to Boolean over the literal
val cell14 = Dial.guard14((true: Boolean).asInstanceOf[Boolean]) // guard, a cast to Boolean over an ascribed literal
val cell15 = Dial.guard15(false.asInstanceOf[true].asInstanceOf[Boolean]) // guard, a cast to Boolean over a cast to the singleton
val cell16 = Dial.guard16({ true.asInstanceOf[Boolean] }) // guard, a block of a cast to Boolean
val cell17 = Dial.byValue17(true.asInstanceOf[Boolean]) // byValue, a cast to Boolean over the literal
val cell18 = Dial.byValue18(true.asInstanceOf[true]) // byValue, a cast to the singleton over its literal
val cell19 = Dial.byValue19(Source.off().asInstanceOf[true]) // byValue, a cast to the singleton over a call
val cell20 = Dial.byValue20((false: Boolean).asInstanceOf[Pins.t.type]) // byValue, a cast to a path's type over an ascribed literal
val cell21 = Dial.byValue21(false.asInstanceOf[Boolean].asInstanceOf[true]) // byValue, a cast to the singleton over a cast to Boolean
val cell22 = Dial.byValue22({ val u = 0; false.asInstanceOf[true] }) // byValue, a block with a statement ending in a cast to the singleton
val cell23 = Dial.byName23((true: Boolean).asInstanceOf[Boolean]) // a by-name parameter, a cast to Boolean
val cell24 = Dial.byName24((false: Boolean).asInstanceOf[true]) // a by-name parameter, a cast to the singleton over an ascribed literal
val cell25 = Dial.byName25(true.asInstanceOf[true]) // a by-name parameter, a cast to the singleton over its literal
val cell26 = Dial.plain26(true.asInstanceOf[true]) // a by-value parameter, a cast to the singleton over its literal
val cell27 = Dial.plain27(Pins.held()) // a by-value parameter, a call of a singleton result type
val cell28 = Dial.plain28(Pins.quiet()) // a by-value parameter, a call of a singleton result type with no effect
