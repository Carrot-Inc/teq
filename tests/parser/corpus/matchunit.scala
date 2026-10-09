sealed trait Event
case class Click(x: Int) extends Event
case class Key(code: String) extends Event
case object Tick extends Event

class Counter:
  var clicks = 0
  var keys: List[String] = Nil
  var ticks = 0
  var log = ""

  def handle(e: Event): Unit = e match
    case Click(x) => clicks += x
    case Key(code) => keys = code :: keys
    case Tick => ticks += 1

  def handleNested(e: Event): Unit = e match
    case Click(x) =>
      if x > 1 then clicks += 100
    case Key(code) =>
      var i = 0
      while i < 2 do
        log = log + code
        i += 1
    case Tick =>
      if ticks > 0 then log = log + "t" else log = log + "T"

@main def run(): Unit =
  val c = Counter()
  List(Click(2), Key("a"), Tick, Click(3), Key("b")).foreach(c.handle)
  println((c.clicks, c.keys, c.ticks))
  List(Click(1), Click(2), Key("k"), Tick).foreach(c.handleNested)
  println((c.clicks, c.log))
  def visit(x: Int | String): Unit = x match
    case i: Int => c.clicks = i
    case s: String => c.log = s
  visit(7)
  visit("done")
  println((c.clicks, c.log))
