// The prototype a view search of an application's function reveals is the function's
// (`FunProto`), shown with its arguments and result (RefinedPrinter.scala:356-361),
// in `TooUnspecific`'s note (Implicits.scala:1684-1686; scalac 3.8.4
// on each line). Queued: push 6 (as extension_view_too_unspecific).
trait Ops:
  extension (r: Any) def pick(i: Int)(j: Int): String
object Main:
  given a: Ops with
    extension (r: Any) def pick(i: Int)(j: Int): String = "a"
  def main(args: Array[String]): Unit =
    println((??? : Nothing).pick(1)(args.length))
    val s: String = (null: Null).pick(2)
