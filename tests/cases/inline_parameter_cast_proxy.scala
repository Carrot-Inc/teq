// A parameter that is not `inline`, by value or by name, takes no constant from a cast written on
// its argument: its reference is a proxy of the widened type (`Inliner.paramBindingDef`: a written
// `asInstanceOf` is no idempotent path). Each line is one reader over one argument the compiler
// does not evaluate: a call with an effect, a local of the caller, its parameter, a field, a lazy
// local. In a transparent method that holds whatever the cast's type and whatever stands under
// it; a plain inline method, which the inlining phase expands, is here with the arguments under a
// cast to a singleton type.
object Tap:
  def off(): Boolean = { println("evaluated"); false }
object Hold:
  val plain: Boolean = false
object Dial:
  transparent inline def byNameSelector(b: => Boolean): String =
    inline b match
      case true => "taken"
      case false => "other"
      case _ => "neither"
  transparent inline def byValueSelector(b: Boolean): String =
    inline b match
      case true => "taken"
      case false => "other"
      case _ => "neither"
  inline def laterByNameSelector(b: => Boolean): String =
    inline b match
      case true => "taken"
      case false => "other"
      case _ => "neither"
  inline def laterByValueSelector(b: Boolean): String =
    inline b match
      case true => "taken"
      case false => "other"
      case _ => "neither"
  inline def laterByNameChoice(b: => Boolean): String =
    if b then "taken" else "other"
  transparent inline def byNameComponent(b: => Boolean): String =
    inline (b, 1) match
      case (true, 1) => "taken"
      case _ => "neither"

def use(flag: Boolean): Unit =
  val near: Boolean = false
  val sure: true = true
  lazy val late: Boolean = Tap.off()
  println("byNameSelector, a call with an effect under a cast to the singleton: " + Dial.byNameSelector(Tap.off().asInstanceOf[true]))
  println("byNameSelector, a local of the caller under a cast to the singleton: " + Dial.byNameSelector(near.asInstanceOf[true]))
  println("byNameSelector, the caller's parameter under a cast to the singleton: " + Dial.byNameSelector(flag.asInstanceOf[true]))
  println("byNameSelector, a field under a cast to the singleton: " + Dial.byNameSelector(Hold.plain.asInstanceOf[true]))
  println("byNameSelector, a lazy local under a cast to the singleton: " + Dial.byNameSelector(late.asInstanceOf[true]))
  println("byNameSelector, a local of a literal type under a cast to Boolean: " + Dial.byNameSelector(sure.asInstanceOf[Boolean]))
  println("byNameSelector, the negation of a local under a cast to the singleton: " + Dial.byNameSelector(!(near.asInstanceOf[true])))
  println("byNameSelector, a block of a local under a cast to the singleton: " + Dial.byNameSelector({ near.asInstanceOf[true] }))
  println("byValueSelector, a call with an effect under a cast to the singleton: " + Dial.byValueSelector(Tap.off().asInstanceOf[true]))
  println("byValueSelector, a local of the caller under a cast to the singleton: " + Dial.byValueSelector(near.asInstanceOf[true]))
  println("byValueSelector, the caller's parameter under a cast to the singleton: " + Dial.byValueSelector(flag.asInstanceOf[true]))
  println("byValueSelector, a field under a cast to the singleton: " + Dial.byValueSelector(Hold.plain.asInstanceOf[true]))
  println("byValueSelector, a lazy local under a cast to the singleton: " + Dial.byValueSelector(late.asInstanceOf[true]))
  println("byValueSelector, a local of a literal type under a cast to Boolean: " + Dial.byValueSelector(sure.asInstanceOf[Boolean]))
  println("byValueSelector, the negation of a local under a cast to the singleton: " + Dial.byValueSelector(!(near.asInstanceOf[true])))
  println("byValueSelector, a block of a local under a cast to the singleton: " + Dial.byValueSelector({ near.asInstanceOf[true] }))
  println("laterByNameSelector, a call with an effect under a cast to the singleton: " + Dial.laterByNameSelector(Tap.off().asInstanceOf[true]))
  println("laterByNameSelector, a local of the caller under a cast to the singleton: " + Dial.laterByNameSelector(near.asInstanceOf[true]))
  println("laterByNameSelector, the caller's parameter under a cast to the singleton: " + Dial.laterByNameSelector(flag.asInstanceOf[true]))
  println("laterByNameSelector, a field under a cast to the singleton: " + Dial.laterByNameSelector(Hold.plain.asInstanceOf[true]))
  println("laterByNameSelector, a lazy local under a cast to the singleton: " + Dial.laterByNameSelector(late.asInstanceOf[true]))
  println("laterByNameSelector, the negation of a local under a cast to the singleton: " + Dial.laterByNameSelector(!(near.asInstanceOf[true])))
  println("laterByNameSelector, a block of a local under a cast to the singleton: " + Dial.laterByNameSelector({ near.asInstanceOf[true] }))
  println("laterByValueSelector, a call with an effect under a cast to the singleton: " + Dial.laterByValueSelector(Tap.off().asInstanceOf[true]))
  println("laterByValueSelector, a local of the caller under a cast to the singleton: " + Dial.laterByValueSelector(near.asInstanceOf[true]))
  println("laterByValueSelector, the caller's parameter under a cast to the singleton: " + Dial.laterByValueSelector(flag.asInstanceOf[true]))
  println("laterByValueSelector, a field under a cast to the singleton: " + Dial.laterByValueSelector(Hold.plain.asInstanceOf[true]))
  println("laterByValueSelector, a lazy local under a cast to the singleton: " + Dial.laterByValueSelector(late.asInstanceOf[true]))
  println("laterByValueSelector, the negation of a local under a cast to the singleton: " + Dial.laterByValueSelector(!(near.asInstanceOf[true])))
  println("laterByValueSelector, a block of a local under a cast to the singleton: " + Dial.laterByValueSelector({ near.asInstanceOf[true] }))
  println("laterByNameChoice, a call with an effect under a cast to the singleton: " + Dial.laterByNameChoice(Tap.off().asInstanceOf[true]))
  println("laterByNameChoice, a local of the caller under a cast to the singleton: " + Dial.laterByNameChoice(near.asInstanceOf[true]))
  println("laterByNameChoice, the caller's parameter under a cast to the singleton: " + Dial.laterByNameChoice(flag.asInstanceOf[true]))
  println("laterByNameChoice, a field under a cast to the singleton: " + Dial.laterByNameChoice(Hold.plain.asInstanceOf[true]))
  println("laterByNameChoice, a lazy local under a cast to the singleton: " + Dial.laterByNameChoice(late.asInstanceOf[true]))
  println("laterByNameChoice, a local of a literal type under a cast to Boolean: " + Dial.laterByNameChoice(sure.asInstanceOf[Boolean]))
  println("laterByNameChoice, a block of a local under a cast to the singleton: " + Dial.laterByNameChoice({ near.asInstanceOf[true] }))
  println("byNameComponent, a call with an effect under a cast to the singleton: " + Dial.byNameComponent(Tap.off().asInstanceOf[true]))
  println("byNameComponent, a local of the caller under a cast to the singleton: " + Dial.byNameComponent(near.asInstanceOf[true]))
  println("byNameComponent, the caller's parameter under a cast to the singleton: " + Dial.byNameComponent(flag.asInstanceOf[true]))
  println("byNameComponent, a field under a cast to the singleton: " + Dial.byNameComponent(Hold.plain.asInstanceOf[true]))
  println("byNameComponent, a lazy local under a cast to the singleton: " + Dial.byNameComponent(late.asInstanceOf[true]))
  println("byNameComponent, a local of a literal type under a cast to Boolean: " + Dial.byNameComponent(sure.asInstanceOf[Boolean]))
  println("byNameComponent, the negation of a local under a cast to the singleton: " + Dial.byNameComponent(!(near.asInstanceOf[true])))
  println("byNameComponent, a block of a local under a cast to the singleton: " + Dial.byNameComponent({ near.asInstanceOf[true] }))

@main def run(): Unit = use(false)
