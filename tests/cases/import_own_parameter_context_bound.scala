// A context bound's evidence named by an import inside the class's parent argument is the constructor's parameter
// there too (dotty's `Desugar` makes the evidence a parameter accessor of the primary constructor): imported by its
// member's name and by a wildcard, in a class, in an enum case, in a given with a body and in a class that extends
// the given's class. teq read it on a module of the class, which has none. The evidence written there, imported in
// the class's body, and a using clause's parameter imported in the parent's argument, stay as they were.

trait Show[T] { def show(t: T): String; def tag(k: Int): String = "tag" + k; extension (t: T) def shown: String = show(t) }
given Show[Int] = (t: Int) => "int" + t
class Base(val s: String)
class Box[T](val s: String)

class Named[T: {Show as sh}](x: T) extends Base({ import sh.show; show(x) })
class Wild[T: {Show as sh}](x: T) extends Base({ import sh.*; x.shown + show(x) })
enum Kind(val s: String) { case Of[T: {Show as sh}](x: T) extends Kind({ import sh.show; show(x) }) }
given boxed[T: {Show as sh}]: Box[T]({ import sh.*; "given " + tag(1) }) with {}
class Plain[T: {Show as sh}] extends Box[T]({ import sh.tag; "plain " + tag(2) })
class Written[T: {Show as sh}](x: T) extends Base(sh.show(x))
class InBody[T: {Show as sh}](x: T) extends Base("body") { def f: String = { import sh.*; x.shown } }
class Using[T](x: T)(using sh: Show[T]) extends Base({ import sh.*; x.shown })

@main def run(): Unit =
  println(new Named(2).s)
  println(new Wild(3).s)
  println(Kind.Of(4).s)
  println(summon[Box[Int]].s)
  println(new Plain[Int].s)
  println(new Written(5).s)
  println(new InBody(6).f)
  println(new Using(7).s)
