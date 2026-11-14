// A cast on an `inline` parameter's argument reaches the reader of a transparent method through
// what stands between: a transparent method that passes its own parameter on, or casts it; an
// argument with an effect, evaluated where each read stands. A parameter that is not `inline`, by
// value or by name, takes no constant from a cast: its reference is a proxy of the widened type
// (`Inliner.paramBindingDef`).
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
  transparent inline def fwdIf(inline c: Boolean): Int = inlineIf(c)
  transparent inline def fwdSelector(inline c: Boolean): String = selector(c)
  transparent inline def fwdComponent(inline c: Boolean): String = component(c)
  transparent inline def fwdByValue(inline c: Boolean): Int = byValue(c)
  transparent inline def castIf(inline c: Boolean): Int = inlineIf(c.asInstanceOf[true])
  transparent inline def castSelector(inline c: Boolean): String = selector(c.asInstanceOf[true])
  transparent inline def wideSelector(inline c: Boolean): String = selector(c.asInstanceOf[Boolean])
  transparent inline def byNameSelector(b: => Boolean): String =
    inline b match
      case true => "nine"
      case _ => "other"
  transparent inline def plainSelector(b: Boolean): String =
    inline b match
      case true => "nine"
      case _ => "other"
  transparent inline def twice(inline b: Boolean): Int = (if b then 1 else 0) + (if b then 10 else 0)
  transparent inline def size(inline n: Int): String =
    inline n match
      case 9 => "nine"
      case -9 => "minus"
      case 10 => "ten"
      case _ => "other"

@main def run(): Unit =
  println("a forwarder passes a cast to the singleton on to inlineIf: " + Dial.fwdIf((false: Boolean).asInstanceOf[true]))
  println("a forwarder passes a cast over a call on to inlineIf: " + Dial.fwdIf(Source.off().asInstanceOf[true]))
  println("a forwarder passes a cast to a path's type on to inlineIf: " + Dial.fwdIf((false: Boolean).asInstanceOf[Pins.t.type]))
  println("a forwarder passes a cast to Boolean on to the selector: " + Dial.fwdSelector(true.asInstanceOf[Boolean]))
  println("a forwarder passes a cast to the singleton on to the selector: " + Dial.fwdSelector((false: Boolean).asInstanceOf[true]))
  println("a forwarder passes a cast to Boolean on to the component: " + Dial.fwdComponent((true: Boolean).asInstanceOf[Boolean]))
  println("a forwarder passes a cast over a call on to the component: " + Dial.fwdComponent(Source.off().asInstanceOf[true]))
  println("a forwarder casts its parameter to the singleton for inlineIf: " + Dial.castIf((false: Boolean)))
  println("a forwarder casts a call to the singleton for inlineIf: " + Dial.castIf(Source.off()))
  println("a forwarder casts its parameter to the singleton for the selector: " + Dial.castSelector((false: Boolean)))
  println("a forwarder casts a literal to Boolean for the selector: " + Dial.wideSelector(true))
  println("a by-name parameter takes no constant from a cast to the singleton: " + Dial.byNameSelector(true.asInstanceOf[true]))
  println("a by-name parameter takes no constant from a cast to Boolean: " + Dial.byNameSelector((true: Boolean).asInstanceOf[Boolean]))
  println("a by-value parameter takes no constant from a cast to the singleton: " + Dial.plainSelector(true.asInstanceOf[true]))
  println("an argument with an effect under a cast, read twice: " + Dial.twice(Tap.on().asInstanceOf[true]))
  println("an Int selector over a cast to the singleton 9: " + Dial.size(8.asInstanceOf[9]))
  println("an Int selector over a cast of a call to 9: " + Dial.size(Source.count().asInstanceOf[9]))
  println("an Int selector over a cast to Int: " + Dial.size(9.asInstanceOf[Int]))
  println("an Int selector over the negation of a cast to 9: " + Dial.size(-(8.asInstanceOf[9])))
  println("an Int selector over a cast to a path's type: " + Dial.size(8.asInstanceOf[Pins.nine.type]))
