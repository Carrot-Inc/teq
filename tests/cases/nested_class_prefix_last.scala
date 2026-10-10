// Classes nested in classes through prefixes, the last fold: a bare extractor inside its class
// takes its own occurrence (`case I(n)` is `O.this.I` inside `O`, `Sub.this.I` in a subclass,
// `b.I` under `import b.I`), not the scrutinee's prefix; a trait of an abstract val is a pure
// interface, whose type pattern takes no outer test; a secondary constructor of an inner
// superclass takes its outer argument (through a path, an alias and the class's own `this`);
// a sealed parent through a prefix has its child in an object of the class.
import scala.deriving.Mirror

object Bare:
  class O:
    case class I(n: Int)
    def test(x: O#I): String = x match
      case I(n) => "wrong"
      case _ => "right"
    class P:
      def test(x: O#I): String = x match
        case I(n) => "wrong"
        case _ => "right"
  class Sub extends O:
    def test2(x: O#I): String = x match
      case I(n) => "same"
      case _ => "other"

object Abstract:
  class O:
    trait T:
      val n: Int

object Secondary:
  class O(val n: Int):
    class I(val x: Int):
      def this() = this(3)
      def this(s: String) = this(s.length)
      def value = n + x
    class J extends I()
    class K extends I("abcd"):
      def this(k: Int) = this()
    def mk = new J().value + new K(0).value
  object H:
    val o = new O(7)
    type Base = o.I
  class C extends H.o.I()
  class D extends H.Base("ab")
  class P(val m: Int):
    class Q extends H.o.I():
      def both = m + value

object DeepMirror:
  class O:
    class Mid:
      class Deep:
        sealed trait T
      val deep = new Deep
    val mid = new Mid
    object Kids:
      case class C(n: Int) extends mid.deep.T

@main def run(): Unit =
  val a = new Bare.O
  val b = new Bare.O
  println(a.test(new b.I(1)) + " " + new a.P().test(new b.I(1)) + " " + a.test(new a.I(1)))
  val s = new Bare.Sub
  println(s.test2(new b.I(1)) + " " + s.test2(new s.I(1)))
  locally {
    import b.I
    val x: Bare.O#I = new a.I(1)
    println(x match
      case I(n) => "wrong"
      case _ => "right")
  }

  val ao = new Abstract.O
  val bo = new Abstract.O
  val t: Any = new bo.T { val n = 2 }
  println(t match
    case _: ao.T => "matched"
    case _ => "missed")

  println(new Secondary.C().value + " " + new Secondary.D().value + " " + Secondary.H.o.mk)
  val p = new Secondary.P(100)
  println(new p.Q().both)
  val local = new Secondary.O(1)
  class L extends local.I()
  println(new L().value)

  val o = new DeepMirror.O
  val m = summon[Mirror.SumOf[o.mid.deep.T]]
  summon[m.MirroredElemTypes =:= Tuple1[o.Kids.C]]
  println(m.ordinal(o.Kids.C(3)))
