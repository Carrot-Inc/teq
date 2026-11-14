// A plain `if` over an `inline` parameter whose argument is a cast to `Boolean` is not reduced:
// its other branch is kept for the inlining phase, which reports it (each cell a reader of its
// own). The typer reports nothing here, so the phase runs.
// expect: 34:13: error: the other branch is kept
// expect: 35:13: error: the other branch is kept
// expect: 36:13: error: the other branch is kept
// expect: 37:13: error: the other branch is kept
// expect: 38:13: error: the other branch is kept
// expect: 5 errors found
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
  transparent inline def plainIf0(inline b: Boolean): Int =
    if b then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def plainIf1(inline b: Boolean): Int =
    if b then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def plainIf2(inline b: Boolean): Int =
    if b then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def plainIf3(inline b: Boolean): Int =
    if b then 100 else scala.compiletime.error("the other branch is kept")
  transparent inline def plainIf4(inline b: Boolean): Int =
    if b then 100 else scala.compiletime.error("the other branch is kept")

val cell0 = Dial.plainIf0(true.asInstanceOf[Boolean]) // plainIf, a cast to Boolean over the literal
val cell1 = Dial.plainIf1(Source.on().asInstanceOf[Boolean]) // plainIf, a cast to Boolean over a call
val cell2 = Dial.plainIf2(false.asInstanceOf[true].asInstanceOf[Boolean]) // plainIf, a cast to Boolean over a cast to the singleton
val cell3 = Dial.plainIf3({ true.asInstanceOf[Boolean] }) // plainIf, a block of a cast to Boolean
val cell4 = Dial.plainIf4(!(true.asInstanceOf[true])) // plainIf, the negation of a cast to the singleton
