// A parameterless method that overrides a Java method declared with `()` is called either way, as dotty's
// `matchNullaryLoosely` asks it of the member's overridden symbols: a class's own `length` over `CharSequence`'s and,
// on the JVM, scala-library's `StringBuilder.length` (master: "method length in class StringBuilder does not take
// parameters"); a parameterless method overriding none still refuses `()` (tests/errors/nullary_parens_refused).
class Mine extends CharSequence:
  def length: Int = 3
  def charAt(i: Int): Char = 'a'
  def subSequence(a: Int, b: Int): CharSequence = this
@main def run() =
  println(new Mine().length())
  println(new Mine().length)
  val sb = new StringBuilder("x")
  println(sb.length())
  println(sb.length)
  val cs: CharSequence = sb
  println(cs.length())
  println(new java.lang.StringBuilder("ab").length())
