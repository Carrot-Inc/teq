// A class's own constructor parameter imported inside its parent's argument, or inside a default of a later
// parameter clause (dotty's default getter takes the earlier clauses' parameters as its own), and captured there by
// what outlives the construction: a closure, a local class or method, nested closures, an anonymous class, a SAM's
// function literal, a local class's export, a local class's own parent call, a by-name parent argument. Each reads
// the constructor's argument, the parameter and not the member: a subclass that overrides the member changes what the
// class's body reads, not what the closure captured. teq read the parameter on a module of the class, which has none.

class Dsl(val n: Int) { def twice(k: Int): Int = n * k }
class Fn(val f: () => Int)
class Base(val s: String)
trait Job { def run(): Int }
class Lazy(i: => Int) { def run(): Int = i }

class Lambda(d: Dsl) extends Fn({ import d.twice; () => twice(3) })
class Saving(held: Dsl) extends Fn({ import held.twice; () => twice(3) }) {
  def d: Dsl = held
  def field(): Int = d.twice(3)
}
class Overriding extends Saving(new Dsl(2)) { override def d: Dsl = new Dsl(9) }
class LocalClass(d: Dsl) extends Fn({ import d.twice; class Saved { def run(): Int = twice(3) }; val s = new Saved; () => s.run() })
class LocalDef(d: Dsl) extends Fn({ import d.twice; def saved(): Int = twice(3); () => saved() })
class NestedLambda(d: Dsl) extends Fn({ import d.twice; val f = () => () => twice(3); () => f()() })
class Anonymous(d: Dsl) extends Fn({ val s = new Job { def run(): Int = { import d.twice; twice(3) } }; () => s.run() })
class Sam(d: Dsl) extends Fn({ val s: Job = () => { import d.twice; twice(3) }; () => s.run() })
class Exported(d: Dsl) extends Fn({ class Saved { export d.twice }; val s = new Saved; () => s.twice(3) })
class CapturedSuper(d: Dsl) extends Fn({ class L extends Base({ import d.twice; twice(3).toString }); val l = new L; () => l.s.toInt })
class ByNameParent(d: Dsl) extends Lazy({ import d.twice; twice(3) })
class Later(d: Dsl)(val t: String = { import d.twice; twice(3).toString })
class LaterLambda(d: Dsl)(val f: () => Int = { import d.twice; () => twice(3) })
class LaterLocal(d: Dsl)(val f: () => Int = { import d.twice; class Saved { def run(): Int = twice(3) }; val s = new Saved; () => s.run() })
trait LaterTrait(d: Dsl)(val result: Int = { import d.twice; twice(3) })
class LaterImpl extends LaterTrait(new Dsl(2))()
case class LaterCase(d: Dsl)(val result: Int = { import d.twice; twice(3) })

@main def run(): Unit =
  val l = new Lambda(new Dsl(2))
  println(l.f() + l.f())
  val o = new Overriding
  println(s"${o.f()} ${o.field()}")
  println(new LocalClass(new Dsl(2)).f())
  println(new LocalDef(new Dsl(2)).f())
  println(new NestedLambda(new Dsl(2)).f())
  println(new Anonymous(new Dsl(2)).f())
  println(new Sam(new Dsl(2)).f())
  println(new Exported(new Dsl(2)).f())
  println(new CapturedSuper(new Dsl(2)).f())
  val b = new ByNameParent(new Dsl(2))
  println(s"${b.run()} ${b.run()}")
  println(new Later(new Dsl(2))().t)
  println(new LaterLambda(new Dsl(2))().f())
  println(new LaterLocal(new Dsl(2))().f())
  println(new LaterImpl().result)
  val c = LaterCase(new Dsl(2))()
  println(s"${c.result} ${c.copy(d = new Dsl(4))(c.result).result}")
