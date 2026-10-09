// teq: --dialect explicit-result-types
// expect: `size` needs a declared result type under the dialect flag `explicit-result-types`: an inferred type makes the type of a definition depend on its body, which is typed before any use of it, and an edit of the body can change every use
// expect: `count` needs a declared type under the dialect flag `explicit-result-types`
// A private definition and a local one may leave their types to inference.
object O:
  def size(xs: List[Int]) = xs.length
  val count = 3
  private val hidden = 4
  def declared(x: Int): Int =
    val local = x + hidden
    local

@main def main(): Unit =
  println(O.size(List(1)) + O.count + O.declared(1))
