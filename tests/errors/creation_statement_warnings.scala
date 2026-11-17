// teq: --werror
// A creation as a statement or as a dropped value is told where the reference holds its constructor for stable:
// a class of the program that runs no initialiser, `Some` and a tuple class. A class with a body, another class
// of the standard library and a class of the JDK are not told.
class Plain(x: Int)
class Bodied(x: Int) { println("made") }
case class Record(x: Int)
def dropped: Unit = new Plain(2)
def droppedLibrary: Unit = new StringBuilder("t")
def statements: Unit = {
  new Plain(1)
  new Bodied(1)
  new Record(1)
  Record(2)
  new Object
  new java.util.Date(0L)
  new StringBuilder("s")
  new Left[Int, Int](1)
  new ::(1, Nil)
  new scala.util.Success(1)
  new java.util.concurrent.atomic.AtomicLong(1L)
  new Some(1)
  new Tuple2(1, 2)
  Some(3)
  (4, 5)
  ()
}
class Slot[A]
trait Shape { type Elems }
inline def inlined(s: Shape): Unit = { new Slot[s.Elems]; () }
def expanded(s: Shape): Unit = inlined(s)
