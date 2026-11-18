// The places beside a class's own parameter imported in its parent's argument where the import's value was read
// right already, kept as they are: the class's body, its methods, a lazy val and a closure there that outlives the
// constructor (the field, `this`'s), a trait's own parameter, an enum case's body, a secondary constructor's self
// call (its own parameter, a local) and body, a method's default, the enclosing class's parameter in a nested
// class's parent call, a local val of the parameter, a named stable or parameterless member, a named given, an inline
// expansion, a method's parameter, and the selections written on the parameter, a construction `new d.Inner` among
// them.
import scala.language.implicitConversions

class Rich(val text: String) { def shout: String = text.toUpperCase }
class Dsl(val n: Int) {
  implicit def conv(b: Boolean): Rich = new Rich("dsl" + n)
  def helper: Int = n
  def twice(k: Int): Int = n * k
  val three: Int = 3 * n
  extension (i: Int) def plus: Int = i + n
  class Inner(k: Int) { def v: Int = n + k }
}
class Base(val s: String)
class Env(val n: Int) { given rich: Rich = new Rich("env" + n) }
def label(using r: Rich): String = r.text
inline def shown(x: Dsl): String = { import x.twice; twice(3).toString }

class Body(d: Dsl) extends Base("x") {
  import d.*
  val t: String = true.shout + twice(3) + 5.plus + new Inner(4).v
  def f: Int = { import d.twice; twice(3) }
  lazy val l: Int = { import d.twice; twice(4) }
  val g: () => Int = () => { import d.twice; twice(5) }
}
trait Own(d: Dsl) { def f: Int = { import d.twice; twice(3) } }
class OwnImpl extends Own(new Dsl(2))
enum Shape { case Sized(d: Dsl); def f: Int = this match { case Sized(d) => import d.twice; twice(3) } }
class Secondary(s0: String) extends Base(s0) {
  var t: String = ""
  def this(d: Dsl) = this({ import d.twice; twice(3).toString })
  def this(d: Dsl, k: Int) = { this(d); t = { import d.twice; twice(k).toString } }
}
class Defaulted(d: Dsl) extends Base("x") { def f(k: Int = { import d.twice; twice(3) }): Int = k }
class Enclosing(d: Dsl) { class Inside extends Base({ import d.*; true.shout + twice(3) }); def make: Inside = new Inside }
class LocalVal(d: Dsl) extends Base({ val e = d; import e.twice; twice(3).toString })
class NamedStable(d: Dsl) extends Base({ import d.three; import d.helper; three.toString + helper })
class NamedGiven(e: Env) extends Base({ import e.rich; label })
class Inlined(d: Dsl) extends Base(shown(d))
class Written(d: Dsl) extends Base(d.conv(true).shout + d.twice(3) + new d.Inner(4).v)
def method(d: Dsl): String = { import d.*; true.shout + twice(3) + 5.plus + new Inner(4).v + { type I = d.Inner; new I(5).v } }

@main def run(): Unit =
  val b = new Body(new Dsl(2))
  println(b.t + " " + b.f + " " + b.l + " " + b.g())
  println(new OwnImpl().f)
  println(Shape.Sized(new Dsl(2)).f)
  println(new Secondary(new Dsl(2)).s + " " + new Secondary(new Dsl(2), 4).t)
  println(new Defaulted(new Dsl(2)).f())
  println(new Enclosing(new Dsl(2)).make.s)
  println(new LocalVal(new Dsl(2)).s)
  println(new NamedStable(new Dsl(2)).s)
  println(new NamedGiven(new Env(2)).s)
  println(new Inlined(new Dsl(2)).s)
  println(new Written(new Dsl(2)).s)
  println(method(new Dsl(2)))
