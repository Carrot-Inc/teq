// teq: --dialect no-inline
// expect: the call of the inline method `twice` is not allowed under the dialect flag `no-inline`: a call of an inline method types the method's body again at the call site, and the inline calls in that body with it; a plain def is called instead of expanded
object I:
  inline def twice(x: Int): Int = x + x

@main def main(): Unit =
  println(I.twice(2))
