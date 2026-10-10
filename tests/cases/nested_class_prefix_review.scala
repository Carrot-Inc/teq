// Classes nested in classes through prefixes, the second cycle's consumers: a trait parent of an
// instance's member, a constructor pattern through its written prefix (the enclosing class's
// arguments and abstract members seen from it, another instance's case class failing its outer
// test), the children of a sum mirror through `childPrefix`, an extension and a conversion of the
// prefix's class through the implicit scope, a companion's conversions through an object prefix,
// the receiver of an extension through a leading using parameter, and the outer test of type
// patterns, a class's and a trait's.
import scala.deriving.Mirror
import scala.language.implicitConversions

object PrefixedParent:
  class O(val n: Int):
    class Mid:
      trait T:
        def get: Int = n
    val mid = new Mid
    class C extends mid.T

object GenericPattern:
  class O[A]:
    case class I[B](a: A, b: B)

object AbstractPattern:
  class O:
    type A
    case class I(a: A)
  class S extends O:
    type A = Int

object OtherPrefix:
  class O:
    case class I(n: Int)

object SiblingMirror:
  class O:
    class Mid:
      sealed trait T
    val mid = new Mid
    case class C(n: Int) extends mid.T

object ObjectMirror:
  class O:
    sealed trait T
    object Kids:
      case class C(n: Int) extends T

object Anchors:
  class O(val n: Int):
    class I(val x: Int)
    extension (i: I) def total: Int = n + i.x
  class P(val n: Int):
    class I(val x: Int)
    given Conversion[I, Int] with
      def apply(i: I): Int = n + i.x

object CompanionThrough:
  trait T:
    class E
    protected object E:
      implicit def e2i(a: E): Int = 42
    class F
    object F:
      given Conversion[F, Int] = _ => 43
  object Test extends T:
    def both: Int =
      val e: Int = new E()
      val f: Int = new F()
      e + f

object Leading:
  class Ctx(val k: Int):
    class Term
  extension (using c: Ctx)(x: c.Term)
    def same: Int = c.k

object Outer:
  class O(val n: Int):
    class I[A](val a: A)
    trait T:
      def get: Int = n
    def isT(x: Any): Boolean = x match
      case _: T => true
      case _ => false

@main def run(): Unit =
  val o = new PrefixedParent.O(7)
  println(new o.C().get)

  val g = new GenericPattern.O[Int]
  val i = g.I(7, "a")
  val r: Int = i match
    case g.I(a, b) => a + b.length
  println(r)

  val s = new AbstractPattern.S
  val n: Int = s.I(1) match
    case s.I(a) => a
  println(n)

  val a = new OtherPrefix.O
  val b = new OtherPrefix.O
  val x = a.I(1)
  try
    println(x match
      case b.I(k) => k)
  catch case e: MatchError => println("MatchError")

  val so = new SiblingMirror.O
  val sm = summon[Mirror.SumOf[so.mid.T]]
  summon[sm.MirroredElemTypes =:= Tuple1[so.C]]
  println(sm.ordinal(so.C(4)))

  val oo = new ObjectMirror.O
  val om = summon[Mirror.SumOf[oo.T]]
  summon[om.MirroredElemTypes =:= Tuple1[oo.Kids.C]]
  println(om.ordinal(oo.Kids.C(4)))

  val ao = new Anchors.O(10)
  println(new ao.I(3).total)
  val po = new Anchors.P(20)
  val converted: Int = new po.I(3)
  println(converted)

  println(CompanionThrough.Test.both)

  val ctx = new Leading.Ctx(5)
  given c: ctx.type = ctx
  println(new ctx.Term().same)

  val p = new Outer.O(1)
  val q = new Outer.O(2)
  val y: Any = new q.I[Int](1)
  println(y match
    case _: p.I[?] => "wrong"
    case _ => "right")
  println(p.isT(new q.T {}))
  println(p.isT(new p.T {}))
  val ts: List[Any] = List(new p.T {}, new q.T {}, new q.T {})
  println(ts.count { case _: q.T => true; case _ => false })
