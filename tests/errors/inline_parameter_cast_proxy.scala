// The cells of tests/cases/inline_parameter_cast_proxy.scala that scalac refuses, each with a reader
// of its own: an `inline if` on a transparent method's parameter that is not `inline`, by name or
// by value, whose argument is a cast over a tree the compiler does not evaluate: the condition is
// the proxy, no constant, to a singleton type either.
// expect: 52:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 53:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 54:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 55:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 56:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 57:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 58:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 59:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 60:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 61:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 62:16: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 63:16: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 12 errors found
object Tap:
  def off(): Boolean = { println("evaluated"); false }
object Hold:
  val plain: Boolean = false
object Dial:
  transparent inline def nowByName0(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByName1(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByName2(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByName3(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByName4(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByName5(b: => Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByValue6(b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByValue7(b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByValue8(b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByValue9(b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByValue10(b: Boolean): Int =
    inline if b then 100 else 200
  transparent inline def nowByValue11(b: Boolean): Int =
    inline if b then 100 else 200

def use(flag: Boolean): Unit =
  val near: Boolean = false
  val sure: true = true
  lazy val late: Boolean = Tap.off()
  val cell0 = Dial.nowByName0(Tap.off().asInstanceOf[true]) // a by-name parameter, a call with an effect under a cast to the singleton
  val cell1 = Dial.nowByName1(near.asInstanceOf[true]) // a by-name parameter, a local of the caller under a cast to the singleton
  val cell2 = Dial.nowByName2(flag.asInstanceOf[true]) // a by-name parameter, the caller's parameter under a cast to the singleton
  val cell3 = Dial.nowByName3(Hold.plain.asInstanceOf[true]) // a by-name parameter, a field under a cast to the singleton
  val cell4 = Dial.nowByName4(late.asInstanceOf[true]) // a by-name parameter, a lazy local under a cast to the singleton
  val cell5 = Dial.nowByName5(sure.asInstanceOf[Boolean]) // a by-name parameter, a local of a literal type under a cast to Boolean
  val cell6 = Dial.nowByValue6(Tap.off().asInstanceOf[true]) // a by-value parameter, a call with an effect under a cast to the singleton
  val cell7 = Dial.nowByValue7(near.asInstanceOf[true]) // a by-value parameter, a local of the caller under a cast to the singleton
  val cell8 = Dial.nowByValue8(flag.asInstanceOf[true]) // a by-value parameter, the caller's parameter under a cast to the singleton
  val cell9 = Dial.nowByValue9(Hold.plain.asInstanceOf[true]) // a by-value parameter, a field under a cast to the singleton
  val cell10 = Dial.nowByValue10(late.asInstanceOf[true]) // a by-value parameter, a lazy local under a cast to the singleton
  val cell11 = Dial.nowByValue11(sure.asInstanceOf[Boolean]) // a by-value parameter, a local of a literal type under a cast to Boolean
