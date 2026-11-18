// A class's own constructor parameter named by an import inside its parent's argument is the constructor's
// parameter there, as the name itself is (dotty's `Contexts.superCallContext`: the parameter accessors in a scope of
// the primary constructor, read as its parameters by `Constructors.intoConstr`), whatever the import brings (a
// conversion, a given, a method, a val, an extension, a class's constructor proxy, a trait's members, a given of an
// object of the value's, a path's type) and wherever the parent call stands (a trait parent, an enum case, a case
// class and its copy, a class nested in a class or in a block, a using, a private or a public parameter, one that
// shadows an inherited member). teq read the parameter on a module of the class, which has none: the JavaScript
// build panicked or its run failed, the interpreter read a null, the JVM's class died.
import scala.language.implicitConversions

class Rich(val text: String) { def shout: String = text.toUpperCase }
class Dsl(val n: Int) {
  implicit def conv(b: Boolean): Rich = new Rich("dsl" + n)
  given tagged: Conversion[Int, Rich] = i => new Rich("given" + n + i)
  def helper: Int = n
  def twice(k: Int): Int = n * k
  val three: Int = 3 * n
  extension (i: Int) def plus: Int = i + n
  class Inner(k: Int) { def v: Int = n + k }
  object R { given rich: Rich = new Rich("inner" + n) }
}
class Holder(val d: Dsl)
class Base(val s: String)
trait Tagged(val label: String)
trait Ops { def triple(k: Int): Int = 3 * k; extension (k: Int) def tripled: Int = 3 * k }
class Env(val n: Int) extends Ops { given rich: Rich = new Rich("env" + n) }
trait Keyed { type Key; val value: Key; def id(k: Key): Key = k }
class Pair[A](val x: A)
class Pinned(val k: Keyed)(val x: k.Key)

class Wild(d: Dsl) extends Base({ import d.*; true.shout })
class GivenSel(d: Dsl) extends Base({ import d.given; 4.shout })
class ByName(d: Dsl) extends Base({ import d.conv; conv(true).shout })
class Method(d: Dsl) extends Base({ import d.twice; twice(3).toString })
class WildVal(d: Dsl) extends Base({ import d.*; three.toString })
class WildDef(d: Dsl) extends Base({ import d.*; helper.toString })
class Ext(d: Dsl) extends Base({ import d.plus; 5.plus.toString })
class Through(h: Holder) extends Base({ import h.d.twice; twice(3).toString })
class Proxy(d: Dsl) extends Base({ import d.Inner; Inner(4).v.toString })
class Summoned(e: Env) extends Base({ import e.given; summon[Rich].text })
class TraitMembers(e: Env) extends Base({ import e.*; (triple(2) + 4.tripled).toString })
class InnerObject(d: Dsl) extends Base({ import d.R.given; summon[Rich].text })
class TraitParent(d: Dsl) extends Tagged({ import d.twice; twice(3).toString })
class Mixed(d: Dsl) extends Base("x") with Tagged({ import d.*; true.shout + twice(3) })
enum Mode(val s: String) { case On(d: Dsl) extends Mode({ import d.twice; twice(3).toString }) }
case class Copied(d: Dsl) extends Base({ import d.twice; twice(3).toString })
class Using(using d: Dsl) extends Base({ import d.twice; twice(3).toString })
class Private(private val d: Dsl) extends Base({ import d.twice; twice(3).toString })
class Public(val d: Dsl) extends Base({ import d.twice; twice(3).toString })
class Outer(val k: Int) { class Nested(d: Dsl) extends Base({ import d.twice; twice(k).toString }) }
class Shadowed(val result: Int) { val d = new Dsl(9) }
class Shadowing(d: Dsl) extends Shadowed({ import d.twice; twice(3) })
class InLambda(d: Dsl) extends Base(List(3).map(k => { import d.twice; twice(k) }).head.toString)
class InLocal(d: Dsl) extends Base({ class K { import d.twice; def f: Int = twice(3) }; new K().f.toString })
class Expected(k: Keyed) extends Pinned(k)({ import k.*; val v: Key = value; id(v) })
class TypeArgs(k: Keyed) extends Base({ import k.*; class Kept extends Pair[Key](id(value)); val v: k.Key = new Kept().x; v.toString })

@main def run(): Unit =
  println(new Wild(new Dsl(1)).s)
  println(new GivenSel(new Dsl(2)).s)
  println(new ByName(new Dsl(3)).s)
  println(new Method(new Dsl(2)).s)
  println(new WildVal(new Dsl(2)).s)
  println(new WildDef(new Dsl(2)).s)
  println(new Ext(new Dsl(2)).s)
  println(new Through(new Holder(new Dsl(2))).s)
  println(new Proxy(new Dsl(2)).s)
  println(new Summoned(new Env(2)).s)
  println(new TraitMembers(new Env(2)).s)
  println(new InnerObject(new Dsl(2)).s)
  println(new TraitParent(new Dsl(2)).label)
  println(new Mixed(new Dsl(2)).label)
  println(Mode.On(new Dsl(2)).s)
  val c = Copied(new Dsl(2))
  println(c.s + " " + c.copy(d = new Dsl(5)).s + " " + c.copy().s)
  println(new Using(using new Dsl(2)).s)
  println(new Private(new Dsl(2)).s)
  val p = new Public(new Dsl(2))
  locally { import p.d.twice; println(p.s + " " + twice(4)) }
  val o = new Outer(3)
  println(new o.Nested(new Dsl(2)).s)
  val sh = new Shadowing(new Dsl(2))
  println(s"${sh.result} ${sh.d.n}")
  println(new InLambda(new Dsl(2)).s)
  println(new InLocal(new Dsl(2)).s)
  locally { class Local(d: Dsl) extends Base({ import d.twice; twice(3).toString }); println(new Local(new Dsl(2)).s) }
  val k = new Keyed { type Key = Int; val value = 4 }
  println(new Expected(k).x)
  println(new TypeArgs(k).s)
