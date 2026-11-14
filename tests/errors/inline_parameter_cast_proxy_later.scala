// An `inline if` on a plain inline method's parameter that is not `inline`, by name or by value,
// whose argument is a cast to a singleton type over a tree the compiler does not evaluate: the
// condition is the proxy, no constant. Each cell has a reader of its own. The inlining phase
// reports them: the typer reports nothing here.
// expect: 46:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 47:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 48:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 49:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 50:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 51:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 52:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 53:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 54:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 55:15: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 10 errors found
object Tap:
  def off(): Boolean = { println("evaluated"); false }
object Hold:
  val plain: Boolean = false
object Dial:
  inline def laterByName0(b: => Boolean): Int =
    inline if b then 100 else 200
  inline def laterByName1(b: => Boolean): Int =
    inline if b then 100 else 200
  inline def laterByName2(b: => Boolean): Int =
    inline if b then 100 else 200
  inline def laterByName3(b: => Boolean): Int =
    inline if b then 100 else 200
  inline def laterByName4(b: => Boolean): Int =
    inline if b then 100 else 200
  inline def laterByValue5(b: Boolean): Int =
    inline if b then 100 else 200
  inline def laterByValue6(b: Boolean): Int =
    inline if b then 100 else 200
  inline def laterByValue7(b: Boolean): Int =
    inline if b then 100 else 200
  inline def laterByValue8(b: Boolean): Int =
    inline if b then 100 else 200
  inline def laterByValue9(b: Boolean): Int =
    inline if b then 100 else 200

def use(flag: Boolean): Unit =
  val near: Boolean = false
  val sure: true = true
  lazy val late: Boolean = Tap.off()
  val cell0 = Dial.laterByName0(Tap.off().asInstanceOf[true]) // a by-name parameter, a call with an effect under a cast to the singleton
  val cell1 = Dial.laterByName1(near.asInstanceOf[true]) // a by-name parameter, a local of the caller under a cast to the singleton
  val cell2 = Dial.laterByName2(flag.asInstanceOf[true]) // a by-name parameter, the caller's parameter under a cast to the singleton
  val cell3 = Dial.laterByName3(Hold.plain.asInstanceOf[true]) // a by-name parameter, a field under a cast to the singleton
  val cell4 = Dial.laterByName4(late.asInstanceOf[true]) // a by-name parameter, a lazy local under a cast to the singleton
  val cell5 = Dial.laterByValue5(Tap.off().asInstanceOf[true]) // a by-value parameter, a call with an effect under a cast to the singleton
  val cell6 = Dial.laterByValue6(near.asInstanceOf[true]) // a by-value parameter, a local of the caller under a cast to the singleton
  val cell7 = Dial.laterByValue7(flag.asInstanceOf[true]) // a by-value parameter, the caller's parameter under a cast to the singleton
  val cell8 = Dial.laterByValue8(Hold.plain.asInstanceOf[true]) // a by-value parameter, a field under a cast to the singleton
  val cell9 = Dial.laterByValue9(late.asInstanceOf[true]) // a by-value parameter, a lazy local under a cast to the singleton
