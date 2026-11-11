import scala.deriving.Mirror
trait Snapshot[A]:
  def numbers: List[Int]
  def current(value: A): Int
object Snapshot:
  inline def derived[A <: Product](using m: Mirror.ProductOf[A]): Snapshot[A] =
    val saved = List(7)
    new Snapshot[A]:
      val numbers = saved
      def current(value: A) = value.productArity + numbers.head
case class Box(value: Int) derives Snapshot
@main def run(): Unit =
  println(summon[Snapshot[Box]].current(Box(1)))
  CaptureChecks.check()
  println(defaultCapture(20).current(Box(2)))
  println(superCapture(30).current(Box(3)))
  val closure = constructorClosure(40)
  println(closure.current(Box(4)))
  println(closure.current(Box(4)))

class LiveFactory(val offset: Int):
  println("factory")
  inline def make(value: Int): Snapshot[Box] =
    val saved = List(value)
    new Snapshot[Box]:
      val numbers = saved
      def current(box: Box) = numbers.map(n => n + offset + box.value).head

object CaptureChecks:
  def check(): Unit =
    val live = new LiveFactory(3).make(5)
    println(live.current(Box(2)))
    var state = 4
    val later = new Snapshot[Box]:
      val numbers = List(1)
      def current(box: Box) =
        def result(extra: Int = state) = extra + box.value
        result()
    state = 9
    println(later.current(Box(3)))

def defaultCapture(n: Int): Snapshot[Box] =
  class Item(val value: Int = n)
  new Snapshot[Box]:
    val numbers = Nil
    def current(box: Box) = new Item().value + box.value

def superCapture(n: Int): Snapshot[Box] =
  trait Base:
    def amount = n
  trait Added extends Base:
    override def amount = super.amount + 1
  new Added with Snapshot[Box]:
    val numbers = Nil
    def current(box: Box) = amount + box.value

def constructorClosure(n: Int): Snapshot[Box] =
  var counter = n
  new Snapshot[Box]:
    val numbers = Nil
    private val next = () => { counter += 1; counter }
    def current(box: Box) = next() + box.value
