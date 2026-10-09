//> using scala 3.8.4

trait Action:
  def run(): String

trait Handler[T]:
  def handle(t: T): String
  def twice(t: T): String = handle(t) + handle(t)

trait Scaled(using val factor: Int):
  def base: Int
  def scaled: Int = base * factor

trait Marker

enum Op:
  case Add, Mul
  def handler(n: Int): Handler[Int] = new Handler[Int]:
    def handle(t: Int): String = this_op match
      case Add => (t + n).toString
      case Mul => (t * n).toString
  private def this_op: Op = this

object Registry:
  val actions: List[Action] = List(1, 2).map(i => new Action { def run(): String = s"lambda$i" })
  val fixed: Action = new Action:
    def run(): String = s"fixed ${actions.size}"
  var log: List[String] = Nil
  val logging: Action = new Action:
    def run(): String =
      log = "ran" :: log
      s"logged ${log.size}"
  val empty = new Marker {}

def makeAll(names: List[String]): List[Action] =
  for
    n <- names
    upper = n.toUpperCase
  yield new Action:
    def run(): String = s"$n/$upper"

def byNameCapture(msg: => String): Action = new Action:
  def run(): String = msg + msg

def viaLambda(): () => Action =
  var count = 0
  () =>
    count += 1
    val snapshot = count
    new Action:
      def run(): String = s"$snapshot of $count"

def scaledBy(k: Int): Scaled =
  given Int = k
  new Scaled:
    def base: Int = 7

def explicitEvidence(): Scaled = new Scaled(using 3):
  def base: Int = 5

def chain(depth: Int): Action = new Action:
  val label = s"d$depth"
  def run(): String =
    if depth == 0 then label
    else
      val inner = new Action:
        def run(): String = label + ">" + chain(depth - 1).run()
      inner.run()

def handlerWithHelpers(prefix: String): Handler[String] = new Handler[String]:
  private def wrap(s: String): String = s"[$s]"
  val sep: String = ":"
  def handle(t: String): String = wrap(prefix + sep + t)

class Widget(name: String):
  private var clicks = 0
  def onClick: Action = new Action:
    def run(): String =
      clicks += 1
      s"$name clicked $clicks"
  def nested: Action = new Action:
    def run(): String =
      val deeper = new Action:
        def run(): String = s"deep $name $clicks"
      deeper.run()

@main def run(): Unit =
  println(Op.Add.handler(10).twice(5))
  println(Op.Mul.handler(10).handle(5))
  println(Registry.actions.map(_.run()))
  println(Registry.fixed.run())
  println(Registry.logging.run())
  println(Registry.logging.run())
  println(Registry.empty.isInstanceOf[Marker])
  println(makeAll(List("a", "b")).map(_.run()))
  var evaluated = 0
  val bn = byNameCapture { evaluated += 1; s"m$evaluated" }
  println(bn.run())
  println(evaluated)
  val factory = viaLambda()
  val first = factory()
  val second = factory()
  println(first.run())
  println(second.run())
  println(scaledBy(6).scaled)
  println(explicitEvidence().scaled)
  println(chain(2).run())
  println(handlerWithHelpers("p").twice("x"))
  val w = Widget("w")
  println(w.onClick.run())
  println(w.onClick.run())
  println(w.nested.run())
