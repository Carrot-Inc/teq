// A class nested in a local class reads the enclosing method's locals the local class captures:
// a deferred given's synthesised initialiser, a lazy val, a parameter, a var, a local def and the
// enclosing instance, through members, objects, nested members, case classes, subclasses and the
// anonymous, local and lambda-made creators inside the local class. Two instances escape with
// their own values; the lazy given is forced once per instance, when first read.
import scala.compiletime.deferred

trait Need:
  def value: String

trait Deferred:
  given value: String = deferred

class Host(tag: String):
  def show(x: String) = tag + x
  def make(s: String): () => String =
    class HostOuter:
      class Inner:
        def value: String = show(s)
      def inner = new Inner
    val i = new HostOuter().inner
    () => i.value

object Main:
  var forced = 0

  def deferredGiven(s: String): () => String =
    given captured: String = { forced += 1; s }
    class GivenOuter:
      class Inner extends Deferred
      def makeInner(): Deferred = new Inner
    val n = new GivenOuter().makeInner()
    () => n.value

  def lazyMember(s: String): () => String =
    given captured: String = s
    class LazyOuter:
      class Inner extends Need:
        lazy val value: String = captured
      def makeInner(): Need = new Inner
    val n = new LazyOuter().makeInner()
    () => n.value

  def shapes(s: String, k: Int): () => String =
    var hits = 0
    def tag(x: String): String = s"<$x:$k>"
    class Outer:
      object Obj extends Need:
        def value: String = s + "/obj" + k
      case class P(n: Int) extends Need:
        def value: String = { hits += 1; tag(s + n) }
      class Base(val m: Int) extends Need:
        def value: String = s + m
      class Sub extends Base(k):
        override def value: String = "sub" + super.value + tag("q")
      class Mid:
        class Deep(n: Int) extends Need:
          def value: String = s"$s/$n/$k"
        def deep(n: Int): Need = new Deep(n)
      def viaAnon: Need = new Need:
        def value: String = new Base(2).value
      def viaAnonSub: Need = new Base(3):
        override def value: String = "anon" + super.value
      def viaLocalSub: Need =
        class LocSub extends Base(4):
          override def value: String = "loc" + super.value
        new LocSub
      def all: List[Need] =
        val p = P(1)
        List(Obj, p, p.copy(n = 2), new Sub, new Mid().deep(5), viaAnon, viaAnonSub, viaLocalSub)
          ++ List(7, 8).map(i => new Mid().deep(i))
    object Loc:
      class In extends Need:
        def value: String = s + "/loc"
      def in: Need = new In
    val o = new Outer
    () => (o.all :+ Loc.in).map(_.value).mkString(",") + " " + hits

  // A method's default that makes a nested class, and one that reads a capture itself: both read
  // the instance's captures where the method runs.
  def defaults(s: String): String =
    class Outer:
      class Inner extends Need:
        def value: String = s + "/default"
      def inner(n: Need = new Inner): Need = n
      def plain(t: String = s): String = t + "/plain"
    val o = new Outer
    o.inner().value + " " + o.plain()

  // A local class whose one nested definition is an object.
  def onlyObject(s: String): Need =
    class Holder:
      object Obj extends Need:
        def value: String = s + "/only"
    new Holder().Obj

  def main(args: Array[String]): Unit =
    println(defaults("first") + " " + defaults("second"))
    println(onlyObject("first").value + " " + onlyObject("second").value)
    val h = new Host("t:")
    val (h1, h2) = (h.make("first"), h.make("second"))
    println(List(h1(), h2(), h1()).mkString(","))
    val (d1, d2) = (deferredGiven("first"), deferredGiven("second"))
    println(forced)
    println(List(d1(), d1(), d2()).mkString(","))
    println(forced)
    val (l1, l2) = (lazyMember("first"), lazyMember("second"))
    println(List(l1(), l2(), l1()).mkString(","))
    val (a, b) = (shapes("first", 1), shapes("second", 2))
    println(a()); println(b()); println(a())
