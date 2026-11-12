// Where the receiver of sam_literal_receiver_shapes is not taken: an unqualified `synchronized`, `toString`,
// `hashCode` or `getClass` in a function literal converted to an interface that stands in a constructor call's
// argument (a parent's, a secondary constructor's, an enum case's, an object's), in an inline method's body
// expanded elsewhere, or in an inline parameter's argument spliced into a literal. Outside a literal of an
// ordinary method or a field's initialiser the name keeps the receiver it had: these print what they printed.
trait Job { def go(): String }
object Run:
  def job(j: Job): String = j.go()
class Base(val j: Job)

class SuperArg(val tag: String):
  class Sub extends Base(() => synchronized { tag })
  def mk(): String = new Sub().j.go()

class SecondaryArg(val tag: String):
  class K(val j: Job):
    def this(s: String) = this(() => synchronized { s + tag })
  def mk(): String = new K("s").j.go()

class TopSub(tag: String) extends Base(() => synchronized { tag })

class TopSecondary(val j: Job):
  def this(s: String) = this(() => synchronized { s })

class LocalSuperArg(val tag: String):
  def local(): String =
    class L extends Base(() => synchronized { tag })
    new L().j.go()

enum E(val j: Job):
  case A extends E(() => synchronized { "a" })
  case B(n: Int) extends E(() => synchronized { "b" })

object Ob extends Base(() => synchronized { "ob" })
class ObjectSuperArg(val tag: String):
  object In extends Base(() => synchronized { tag })

class InlineNested:
  inline def viaNested: String = Run.job(() => Run.job(() => synchronized { "n" }))
  def onThis(): String = viaNested

class InlineInLiteral:
  inline def viaLit: String = Run.job(() => synchronized { "l" })
  def inLiteral(): String = Run.job(() => viaLit)

class InlineFromInner:
  inline def viaLit: String = Run.job(() => synchronized { "l" })
  class In:
    def use(): String = viaLit

class InlineCallsInline:
  inline def callsInline: String = Run.job(() => inner)
  inline def inner: String = Run.job(() => synchronized { "<>" })
  def onThis(): String = callsInline

object Util:
  inline def lock(inline s: String): String = Run.job(() => s)
class Spliced:
  def m(): String = Util.lock(toString.nonEmpty.toString)
  def h(): String = Util.lock((hashCode == hashCode).toString)
  def g(): String = Util.lock((getClass != null).toString)

@main def run(): Unit =
  println(s"a parent's argument: ${new SuperArg("o").mk()}")
  println(s"a secondary constructor's argument: ${new SecondaryArg("o").mk()}")
  println(s"a top-level class's parent's argument: ${new TopSub("t").j.go()}")
  println(s"a top-level class's secondary constructor's argument: ${new TopSecondary("s").j.go()}")
  println(s"a local class's parent's argument: ${new LocalSuperArg("o").local()}")
  println(s"an enum case's argument: ${E.A.j.go() + E.B(1).j.go()}")
  println(s"an object's parent's argument: ${Ob.j.go() + new ObjectSuperArg("o").In.j.go()}")
  println(s"an inline method's body, a literal in a literal: ${new InlineNested().onThis()}")
  println(s"an inline method's body, called in a literal: ${new InlineInLiteral().inLiteral()}")
  println(s"an inline method's body, called from an inner class: ${ { val a = new InlineFromInner; new a.In().use() } }")
  println(s"an inline method's body that calls an inline method: ${new InlineCallsInline().onThis()}")
  println(s"an inline parameter's argument in a literal: ${new Spliced().m()} ${new Spliced().h()} ${new Spliced().g()}")
