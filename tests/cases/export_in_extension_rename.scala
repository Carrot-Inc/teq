// dotty's tests/pos/export-in-extension-rename.scala, run: `export ops.map` inside `extension [B](x: List[B])`
// makes `map` an extension method over `[B](x: List[B])` whose body is `ops(x).map(...)`, the member's own
// type parameter freshened (`Namer.exportForwarders`, `addPathMethodParams`); `size` likewise.
class Ops[A](xs: List[A]):
  def map[B](x: A => B): List[B] = xs.map(x)
  def size: Int = xs.size
extension [B](x: List[B])
  private def ops = new Ops[B](x)
  export ops.{map, size}
@main def run(): Unit = println(List(1, 2).size + List(1).map(_ + 1).length)
