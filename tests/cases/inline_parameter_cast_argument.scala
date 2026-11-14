// An `inline` parameter of a transparent method stands for its argument's tree: a cast written on
// the argument is read where the parameter is read, by the reader's own rule, as scalac reads it
// (the binding's right side is the argument, `InlineableArg`). Each line is one reader over one
// argument: a cast to a singleton type is that constant whatever stands under it; a cast to
// `Boolean` is no constant, so a selector, a guard and a component take their other case and a
// plain `if` stays an `if`.
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
  transparent inline def inlineIf(inline b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def plainIf(inline b: Boolean): Int =
    if b then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def operand(inline b: Boolean): Int =
    inline if !b then 200 else 100
  transparent inline def selector(inline b: Boolean): String =
    inline b match
      case true => "nine"
      case _ => "other"
  transparent inline def guard(inline b: Boolean): String =
    inline 1 match
      case _ if b => "nine"
      case _ => "other"
  transparent inline def component(inline b: Boolean): String =
    inline (b, 3) match
      case (true, 3) => "nine"
      case _ => "other"
  transparent inline def byValue(inline b: Boolean): Int =
    inline if passV(b) then 100 else 200

@main def run(): Unit =
  println("inlineIf, a cast to the singleton over its literal: " + Dial.inlineIf(true.asInstanceOf[true]))
  println("inlineIf, a cast to the singleton over the other literal: " + Dial.inlineIf(false.asInstanceOf[true]))
  println("inlineIf, a cast to the singleton over an ascribed literal: " + Dial.inlineIf((false: Boolean).asInstanceOf[true]))
  println("inlineIf, a cast to the singleton over a call: " + Dial.inlineIf(Source.off().asInstanceOf[true]))
  println("inlineIf, a cast to a path's type over an ascribed literal: " + Dial.inlineIf((false: Boolean).asInstanceOf[Pins.t.type]))
  println("inlineIf, a cast to a path's type over a call: " + Dial.inlineIf(Source.off().asInstanceOf[Pins.t.type]))
  println("inlineIf, a cast to the singleton over a cast to Boolean: " + Dial.inlineIf(false.asInstanceOf[Boolean].asInstanceOf[true]))
  println("inlineIf, a block of a cast to the singleton: " + Dial.inlineIf({ false.asInstanceOf[true] }))
  println("inlineIf, a block with a statement ending in a cast to the singleton: " + Dial.inlineIf({ val u = 0; false.asInstanceOf[true] }))
  println("inlineIf, the negation of a cast to the singleton: " + Dial.inlineIf(!(true.asInstanceOf[true])))
  println("plainIf, a cast to the singleton over its literal: " + Dial.plainIf(true.asInstanceOf[true]))
  println("plainIf, a cast to the singleton over the other literal: " + Dial.plainIf(false.asInstanceOf[true]))
  println("plainIf, a cast to the singleton over an ascribed literal: " + Dial.plainIf((false: Boolean).asInstanceOf[true]))
  println("plainIf, a cast to the singleton over a call: " + Dial.plainIf(Source.off().asInstanceOf[true]))
  println("plainIf, a cast to a path's type over an ascribed literal: " + Dial.plainIf((false: Boolean).asInstanceOf[Pins.t.type]))
  println("plainIf, a cast to a path's type over a call: " + Dial.plainIf(Source.off().asInstanceOf[Pins.t.type]))
  println("plainIf, a cast to the singleton over a cast to Boolean: " + Dial.plainIf(false.asInstanceOf[Boolean].asInstanceOf[true]))
  println("plainIf, a block of a cast to the singleton: " + Dial.plainIf({ false.asInstanceOf[true] }))
  println("plainIf, a block with a statement ending in a cast to the singleton: " + Dial.plainIf({ val u = 0; false.asInstanceOf[true] }))
  println("operand, a cast to the singleton over its literal: " + Dial.operand(true.asInstanceOf[true]))
  println("operand, a cast to the singleton over the other literal: " + Dial.operand(false.asInstanceOf[true]))
  println("operand, a cast to the singleton over an ascribed literal: " + Dial.operand((false: Boolean).asInstanceOf[true]))
  println("operand, a cast to the singleton over a call: " + Dial.operand(Source.off().asInstanceOf[true]))
  println("operand, a cast to a path's type over an ascribed literal: " + Dial.operand((false: Boolean).asInstanceOf[Pins.t.type]))
  println("operand, a cast to a path's type over a call: " + Dial.operand(Source.off().asInstanceOf[Pins.t.type]))
  println("operand, a cast to the singleton over a cast to Boolean: " + Dial.operand(false.asInstanceOf[Boolean].asInstanceOf[true]))
  println("operand, a block of a cast to the singleton: " + Dial.operand({ false.asInstanceOf[true] }))
  println("operand, a block with a statement ending in a cast to the singleton: " + Dial.operand({ val u = 0; false.asInstanceOf[true] }))
  println("operand, the negation of a cast to the singleton: " + Dial.operand(!(true.asInstanceOf[true])))
  println("selector, a cast to Boolean over the literal: " + Dial.selector(true.asInstanceOf[Boolean]))
  println("selector, a cast to Boolean over an ascribed literal: " + Dial.selector((true: Boolean).asInstanceOf[Boolean]))
  println("selector, a cast to Boolean over a call: " + Dial.selector(Source.on().asInstanceOf[Boolean]))
  println("selector, a cast to Boolean over a final val: " + Dial.selector(Pins.t.asInstanceOf[Boolean]))
  println("selector, a cast to the singleton over its literal: " + Dial.selector(true.asInstanceOf[true]))
  println("selector, a cast to the singleton over the other literal: " + Dial.selector(false.asInstanceOf[true]))
  println("selector, a cast to the singleton over an ascribed literal: " + Dial.selector((false: Boolean).asInstanceOf[true]))
  println("selector, a cast to the singleton over a call: " + Dial.selector(Source.off().asInstanceOf[true]))
  println("selector, a cast to a path's type over an ascribed literal: " + Dial.selector((false: Boolean).asInstanceOf[Pins.t.type]))
  println("selector, a cast to a path's type over a call: " + Dial.selector(Source.off().asInstanceOf[Pins.t.type]))
  println("selector, a cast to an alias of Boolean: " + Dial.selector((true: Boolean).asInstanceOf[Pins.Wide]))
  println("selector, a cast to Boolean over a cast to the singleton: " + Dial.selector(false.asInstanceOf[true].asInstanceOf[Boolean]))
  println("selector, a cast to the singleton over a cast to Boolean: " + Dial.selector(false.asInstanceOf[Boolean].asInstanceOf[true]))
  println("selector, a block of a cast to Boolean: " + Dial.selector({ true.asInstanceOf[Boolean] }))
  println("selector, a block of a cast to the singleton: " + Dial.selector({ false.asInstanceOf[true] }))
  println("selector, a block with a statement ending in a cast to the singleton: " + Dial.selector({ val u = 0; false.asInstanceOf[true] }))
  println("selector, a block with a statement ending in a cast to Boolean: " + Dial.selector({ val u = 0; true.asInstanceOf[Boolean] }))
  println("selector, a parenthesised cast to Boolean: " + Dial.selector((true.asInstanceOf[Boolean])))
  println("selector, the negation of a cast to the singleton: " + Dial.selector(!(true.asInstanceOf[true])))
  println("guard, a cast to the singleton over its literal: " + Dial.guard(true.asInstanceOf[true]))
  println("guard, a cast to the singleton over the other literal: " + Dial.guard(false.asInstanceOf[true]))
  println("guard, a cast to the singleton over an ascribed literal: " + Dial.guard((false: Boolean).asInstanceOf[true]))
  println("guard, a cast to the singleton over a call: " + Dial.guard(Source.off().asInstanceOf[true]))
  println("guard, a cast to a path's type over an ascribed literal: " + Dial.guard((false: Boolean).asInstanceOf[Pins.t.type]))
  println("guard, a cast to a path's type over a call: " + Dial.guard(Source.off().asInstanceOf[Pins.t.type]))
  println("guard, a cast to the singleton over a cast to Boolean: " + Dial.guard(false.asInstanceOf[Boolean].asInstanceOf[true]))
  println("guard, a block of a cast to the singleton: " + Dial.guard({ false.asInstanceOf[true] }))
  println("guard, a block with a statement ending in a cast to the singleton: " + Dial.guard({ val u = 0; false.asInstanceOf[true] }))
  println("guard, the negation of a cast to the singleton: " + Dial.guard(!(true.asInstanceOf[true])))
  println("component, a cast to Boolean over the literal: " + Dial.component(true.asInstanceOf[Boolean]))
  println("component, a cast to Boolean over an ascribed literal: " + Dial.component((true: Boolean).asInstanceOf[Boolean]))
  println("component, a cast to Boolean over a call: " + Dial.component(Source.on().asInstanceOf[Boolean]))
  println("component, a cast to Boolean over a final val: " + Dial.component(Pins.t.asInstanceOf[Boolean]))
  println("component, a cast to the singleton over its literal: " + Dial.component(true.asInstanceOf[true]))
  println("component, a cast to the singleton over the other literal: " + Dial.component(false.asInstanceOf[true]))
  println("component, a cast to the singleton over an ascribed literal: " + Dial.component((false: Boolean).asInstanceOf[true]))
  println("component, a cast to the singleton over a call: " + Dial.component(Source.off().asInstanceOf[true]))
  println("component, a cast to a path's type over an ascribed literal: " + Dial.component((false: Boolean).asInstanceOf[Pins.t.type]))
  println("component, a cast to a path's type over a call: " + Dial.component(Source.off().asInstanceOf[Pins.t.type]))
  println("component, a cast to an alias of Boolean: " + Dial.component((true: Boolean).asInstanceOf[Pins.Wide]))
  println("component, a cast to Boolean over a cast to the singleton: " + Dial.component(false.asInstanceOf[true].asInstanceOf[Boolean]))
  println("component, a cast to the singleton over a cast to Boolean: " + Dial.component(false.asInstanceOf[Boolean].asInstanceOf[true]))
  println("component, a block of a cast to Boolean: " + Dial.component({ true.asInstanceOf[Boolean] }))
  println("component, a block of a cast to the singleton: " + Dial.component({ false.asInstanceOf[true] }))
  println("component, a block with a statement ending in a cast to the singleton: " + Dial.component({ val u = 0; false.asInstanceOf[true] }))
  println("component, a block with a statement ending in a cast to Boolean: " + Dial.component({ val u = 0; true.asInstanceOf[Boolean] }))
  println("component, a parenthesised cast to Boolean: " + Dial.component((true.asInstanceOf[Boolean])))
  println("component, the negation of a cast to the singleton: " + Dial.component(!(true.asInstanceOf[true])))
