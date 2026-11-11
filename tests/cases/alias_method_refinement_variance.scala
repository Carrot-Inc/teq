// An alias whose right side refines by a method, applied at a narrower and at a wider argument: its parameter's sign
// is the method's (a parameter's type flips it, the result keeps it), so a sink of the wider is a sink of the narrower
// and a source of the narrower a source of the wider.
import scala.reflect.Selectable.reflectiveSelectable

trait A
trait B extends A
type Sink[X] = AnyRef { def put(xs: List[X]): Unit }
type Source[X] = AnyRef { def get(): List[X] }

class Store:
  var seen = 0
  def put(xs: List[A]): Unit = seen += xs.length
  def get(): List[B] = List(new B {})

def sinkDown(s: Sink[A]): Sink[B] = s
def sourceUp(s: Source[B]): Source[A] = s

@main def run(): Unit =
  val store = new Store
  sinkDown(store).put(List(new B {}))
  println(sourceUp(store).get().length)
  println(store.seen)
  println("passed")
