class Cell[A](val index: Int)
trait Source[A]:
  def initial: Cell[A]
  def map(f: Cell[A] => Cell[A]): Int = f(initial).index
def source: Source[?] = new Source[String]:
  def initial: Cell[String] = new Cell[String](7)
@main def closureExpectedSkolem(): Unit =
  println(source.map(cell => cell))
