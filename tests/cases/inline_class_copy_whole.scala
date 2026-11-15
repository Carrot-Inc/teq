// A class that an inline method's body declares is, at each call, a class of its own: with the
// type parameters of the one written and their bounds, its methods' parameters typed by it and
// by its fellows of the body, its overloaded members, and its overrides those of a class
// written at the call: a method implementing a generic parent's beside another of its name
// (reached through the parent too), one in an anonymous class and under an abstract class, a
// result narrowed, `Comparable` of itself; a parameter of its own type, of a nested and of a
// sibling class, an array of itself, a default of itself, a bound that names it or a sibling;
// two alternatives of a trait or a class of the body with one of them overridden, first or
// last declared, and of a generic one; two implemented at once; and a name that two unrelated
// traits define and the class does not, which is overloaded in it: called on the class and
// through each trait, the traits outside the method and of the body, the class anonymous
// (scalac prints `5 6 6 6 -2 2 3 3 3 5 3 3 3`, `5 5 5 5 5 5 5`, `5 9 5`, then the same calls
// at 10: `23 15 15 15 7 11 12 12 12 14 12 12 12`, `14 14 14 14 14 14 14`, `14 18 14`).
trait Op[A] { def put(x: A): Int }
abstract class OpC[A] { def put(x: A): Int }
trait Base { def get(x: Int): Any; def get(x: String): Any = x }
trait Both { def put(x: Int): Int; def put(x: String): Int }
trait M1 { def put(x: Int): Int = 1 }
trait M2 { def put(x: String): Int = x.length }

object Inl:
  inline def bridge(k: Int): Int =
    class Two extends Op[Int]:
      def put(x: String): Int = x.length
      def put(x: Int): Int = x + k
    val own = new Two
    val t: Op[Int] = own
    own.put(1) + t.put(2)
  inline def anon(k: Int): Int =
    val t: Op[Int] = new Op[Int]:
      def put(x: String): Int = x.length
      def put(x: Int): Int = x + k
    t.put(2) + 3
  inline def underClass(k: Int): Int =
    class Two extends OpC[Int]:
      def put(x: String): Int = x.length
      def put(x: Int): Int = x + k
    val t: OpC[Int] = new Two
    t.put(2) + 3
  inline def narrowed(k: Int): Int =
    class Two extends Base:
      def get(x: Int): String = "v" * (x + k)
    val b: Base = new Two
    b.get(2).toString.length + b.get("abc").toString.length
  inline def comparable(k: Int): Int =
    class Two(val v: Int) extends Comparable[Two]:
      def compareTo(o: Two): Int = v - o.v
    val c: Comparable[Two] = new Two(k)
    c.compareTo(new Two(3))
  inline def ownType(k: Int): Int =
    class Two:
      def put(x: Two): Int = k + 1
    val t = new Two
    t.put(t)
  inline def nested(k: Int): Int =
    class Two:
      class In(val v: Int)
      def put(x: In): Int = x.v + k
    val t = new Two
    t.put(new t.In(2))
  inline def nestedOwn(k: Int): Int =
    class Two(val v: Int):
      class In:
        def put(x: Two): Int = x.v
      def get(i: In): Int = i.put(this) + k
    val t = new Two(2)
    t.get(new t.In())
  inline def sibling(k: Int): Int =
    class A(val v: Int)
    class Two:
      def put(x: A): Int = x.v + k
    new Two().put(new A(2))
  inline def array(k: Int): Int =
    class Two(val v: Int):
      def sum(xs: Array[Two]): Int = xs.map(_.v).sum + k
    val t = new Two(2)
    t.sum(Array(t, t))
  inline def default(k: Int): Int =
    class Two(val v: Int):
      def put(x: Two = this): Int = x.v + k
    new Two(2).put()
  inline def methodBound(k: Int): Int =
    class Two(val v: Int):
      def pick[T <: Two](x: T): Int = x.v + k
    val t = new Two(2)
    t.pick(t)
  inline def classBound(k: Int): Int =
    class A0(val v: Int)
    class Box[T <: A0](val t: T):
      def get: Int = t.v + k
    new Box(new A0(2)).get

  inline def firstOfTwo(k: Int): Int =
    trait Parent:
      def put(x: Int): Int = 7000
      def put(x: String): Int = x.length
    class Two extends Parent:
      override def put(x: Int): Int = x + k
    val t: Parent = new Two
    t.put(1) + t.put("abc")
  inline def lastOfTwo(k: Int): Int =
    trait Parent:
      def put(x: String): Int = x.length
      def put(x: Int): Int = 7000
    class Two extends Parent:
      override def put(x: Int): Int = x + k
    val t: Parent = new Two
    t.put(1) + t.put("abc")
  inline def firstOfGeneric(k: Int): Int =
    trait Parent[A]:
      def put(x: A): Int = 7000
      def put(x: String): Int = x.length
    class Two extends Parent[Int]:
      override def put(x: Int): Int = x + k
    val t: Parent[Int] = new Two
    t.put(1) + t.put("abc")
  inline def lastOfGeneric(k: Int): Int =
    trait Parent[A]:
      def put(x: String): Int = x.length
      def put(x: A): Int = 7000
    class Two extends Parent[Int]:
      override def put(x: Int): Int = x + k
    val t: Parent[Int] = new Two
    t.put(1) + t.put("abc")
  inline def ofClass(k: Int): Int =
    class Parent:
      def put(x: Int): Int = 7000
      def put(x: String): Int = x.length
    class Two extends Parent:
      override def put(x: Int): Int = x + k
    val t: Parent = new Two
    t.put(1) + t.put("abc")
  inline def abstractGeneric(k: Int): Int =
    trait Parent[A]:
      def put(x: A): Int
      def put(x: String): Int = x.length
    class Two extends Parent[Int]:
      def put(x: Int): Int = x + k
    val t: Parent[Int] = new Two
    t.put(1) + t.put("abc")
  inline def bothAtOnce(k: Int): Int =
    class Two extends Both:
      def put(x: Int): Int = x + k
      def put(x: String): Int = x.length
    val t: Both = new Two
    t.put(1) + t.put("abc")
  inline def joined(k: Int): Int =
    class Two extends M1 with M2
    val t = new Two
    t.put(1) + t.put("abc") + k
  inline def joinedOfBody(k: Int): Int =
    trait P1 { def put(x: Int): Int = 1 }
    trait P2 { def put(x: String): Int = x.length }
    class Two extends P1 with P2
    val t = new Two
    val p1: P1 = t
    val p2: P2 = t
    t.put(1) + t.put("abc") + p1.put(1) + p2.put("abc") + k
  inline def joinedAnonymous(k: Int): Int =
    val t = new M1 with M2 {}
    t.put(1) + t.put("abc") + k

object Calls:
  import Inl.*
  def members(k: Int): String =
    List(bridge(k), anon(k), underClass(k), narrowed(k), comparable(k), ownType(k), nested(k), nestedOwn(k), sibling(k), array(k), default(k), methodBound(k), classBound(k)).mkString(" ")
  def overrides(k: Int): String =
    List(firstOfTwo(k), lastOfTwo(k), firstOfGeneric(k), lastOfGeneric(k), ofClass(k), abstractGeneric(k), bothAtOnce(k)).mkString(" ")
  def joins(k: Int): String =
    List(joined(k), joinedOfBody(k), joinedAnonymous(k)).mkString(" ")

@main def run(): Unit =
  println(Calls.members(1))
  println(Calls.overrides(1))
  println(Calls.joins(1))
  println(Calls.members(10))
  println(Calls.overrides(10))
  println(Calls.joins(10))
