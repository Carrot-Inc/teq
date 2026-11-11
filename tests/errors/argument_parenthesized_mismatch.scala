object Test {
  def f(s: String): Unit = ()
  val result = f((1))
}
// A parenthesized argument is adapted, and its mismatch reported, at the tree inside it (scalac 3.8.4: 3:18, 0-based).
