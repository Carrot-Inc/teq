// A macro's `Ref` of a case class's companion, the class nested in a class and declaring none: at the
// site, and spliced inside a class the quote makes, where it is the enclosing instance's val read
// through that class's outer (dotty's `This(Store)`), not the innermost `this`.
final class Store(val tag: String):
  case class Entry(n: Int):
    override def toString(): String = s"$tag:$n"
  def atSite: String = M.make[Entry].toString
  def nested: Show = M.nested[Entry]

@main def run(): Unit =
  val s = Store("s")
  println(s.atSite)
  println(s.nested.show)
  val t = Store("t")
  println(t.nested.show)
