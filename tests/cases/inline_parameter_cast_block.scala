// What an `inline` parameter's reference reads of a block: its argument's tree is kept wherever
// a cast stands where a constant is read from (the block's result, a local under the cast), and
// the kept tree is read with its own type, the cast's, where a local definition stands before
// it. A parameter that is not `inline` takes its argument's singleton type only where the
// argument is an idempotent path (`Inliner.computeParamBindings`): a call of a singleton result
// type binds a proxy of the widened type.
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
  println("inlineIf, a block whose result is a local under a cast to the singleton: " + Dial.inlineIf({ val x: Boolean = false; x.asInstanceOf[true] }))
  println("selector, a block whose result is a local under a cast to the singleton: " + Dial.selector({ val x: Boolean = false; x.asInstanceOf[true] }))
  println("component, a block whose result is a local under a cast to the singleton: " + Dial.component({ val x: Boolean = false; x.asInstanceOf[true] }))
  println("inlineIf, a local of a call under a cast to the singleton: " + Dial.inlineIf({ val x = Source.off(); x.asInstanceOf[true] }))
  println("selector, a local of a call under a cast to the singleton: " + Dial.selector({ val x = Source.off(); x.asInstanceOf[true] }))
  println("selector, a local under a cast to a path's type: " + Dial.selector({ val x: Boolean = false; x.asInstanceOf[Pins.t.type] }))
  println("component, a local under a cast to a path's type: " + Dial.component({ val x: Boolean = false; x.asInstanceOf[Pins.t.type] }))
  println("selector, a local under a cast to Boolean: " + Dial.selector({ val x: Boolean = true; x.asInstanceOf[Boolean] }))
  println("component, a local under a cast to Boolean: " + Dial.component({ val x: Boolean = true; x.asInstanceOf[Boolean] }))
  println("inlineIf, a block with a local method, ending in a cast to the singleton: " + Dial.inlineIf({ def f(): Boolean = false; f().asInstanceOf[true] }))
  println("selector, a block with a local method, ending in a cast to the singleton: " + Dial.selector({ def f(): Boolean = false; f().asInstanceOf[true] }))
  println("inlineIf, a lazy local under a cast to the singleton: " + Dial.inlineIf({ lazy val z = Source.off(); z.asInstanceOf[true] }))
  println("selector, a lazy local under a cast to the singleton: " + Dial.selector({ lazy val z = Source.off(); z.asInstanceOf[true] }))
  println("inlineIf, a block with a statement, ending in a cast of a call: " + Dial.inlineIf({ val w = 0; Source.off().asInstanceOf[true] }))
  println("component, a block with a statement, ending in a cast of a call: " + Dial.component({ val w = 0; Source.off().asInstanceOf[true] }))
  println("selector, a block with a local method, ending in a cast to Boolean: " + Dial.selector({ def f(): Boolean = true; f().asInstanceOf[Boolean] }))
  println("a by-value parameter given a call of a singleton result type: the proxy is evaluated: " + Dial.plainSelector(Pins.held()))
  println("a by-value parameter given a call of a singleton result type takes no constant: " + Dial.plainSelector(Pins.quiet()))
  println("a by-value parameter given a final val's path takes its constant: " + Dial.plainSelector(Pins.t))
  println("a by-value parameter given a block ending in a literal takes its constant: " + Dial.plainSelector({ val w = 0; true }))
