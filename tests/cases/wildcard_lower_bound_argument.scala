// An argument of a parameter whose type is a lower-bounded wildcard argument of the receiver's type
// (`add` of a `JList[? >: Integer]`, a `Flow.Subscriber[_ >: Int]`'s `onNext`): typed and converted against the
// wildcard's lower bound, as dotty's `asSeenFrom` approximates the wildcard in the parameter's contravariant
// position (`AsSeenFromMap`'s ranges), so that `add(1)` converts the `Int` to an `Integer`.
import java.util.{ArrayList, List as JList}

trait Sink[T]:
  def put(t: T): Unit

def addOne(xs: JList[? >: Integer]): Unit = xs.add(1)
def feed(s: Sink[_ >: String]): Unit = s.put("fed")

@main def main(): Unit =
  val numbers = new ArrayList[Number]()
  addOne(numbers)
  numbers.add(2.5)
  println(numbers)
  val seen = new ArrayList[Any]()
  feed(new Sink[Any] { def put(t: Any): Unit = seen.add(t) })
  println(seen)
