// A case class has no constructor proxy (dotty's `NamerOps.needsConstructorProxies`: `Case` is among
// `NoConstructorProxyNeededFlags`): its name applied, and its `apply` written, named as a value or reached by a
// placeholder, is the companion's synthesized `apply`, of the primary constructor's parameters and nothing else
// (`Desugar.classDef`). What a secondary constructor would take is tupled for the primary's one parameter where
// that holds a tuple, and arguments that fit the primary and a secondary of its arity go to the primary. A `new`,
// a parent call, an anonymous class and a plain class's proxy reach every constructor.
case class G[T](x: T):
  def this(a: T, b: T) = this(a)
case class A(x: Any):
  def this(a: Int, b: Int) = this(a + b)
case class P(x: Product):
  def this(a: Int, b: Int) = this(Tuple1(a + b))
case class T(x: (Int, Int)):
  def this(a: Int, b: Int) = this((a + b, 0))
case class U(x: Int | (Int, Int)):
  def this(a: Int, b: Int) = this(a + b)
case class R(xs: Int*):
  def this(a: Int, b: Int) = this((a + b) :: Nil*)
case class V(x: Int):
  def this(a: Int, b: Int) = this(a + b)
class W(val x: Int):
  def this(a: Int, b: Int) = this(a + b)
class Sub extends V(1, 2)
// Secondaries of the primary's arity, each more specific than it or beside a default.
case class S(x: Any):
  def this(s: String) = this(s.length)
case class L(x: Long):
  def this(i: Int) = this(i.toLong + 100)
case class Df(x: Int, y: Int = 10):
  def this(a: Int) = this(a, 99)
case class Vr(xs: Int*):
  def this(a: Int) = this(a, a)
case class Cc(x: Any)(val y: Int):
  def this(a: Int, b: Int) = this((a, b))(0)
object O:
  case class K(x: Any):
    def this(a: Int, b: Int) = this(a + b)
trait Outer:
  case class In(x: Any):
    def this(a: Int, b: Int) = this(a + b)
object D extends Outer
class C extends Outer:
  def make: In = In(1, 2)
def twice[X](f: X => V, a: X): V = f(a)

@main def main(): Unit =
  // The synthesized `apply`: the primary alone, the arguments tupled.
  println(G(1, 2))
  println(A(1, 2))
  println(A.apply(1, 2))
  println(P(1, 2))
  println(T(1, 2))
  println(U(1, 2))
  println(R(1, 2).xs.mkString(","))
  val pair: (Int, Int) => A = A(_, _)
  println(pair(1, 2))
  println(O.K(1, 2))
  println(D.In(1, 2))
  val c = new C
  println(c.In(1, 2))
  println(c.make)
  import D.In
  println(In(1, 2))
  // Arguments a secondary of the primary's arity would take more specifically are the primary's.
  println(S("abc"))
  println(L(1))
  println(Df(1))
  println(Vr(1).xs.mkString(","))
  println(List("abc").map(S(_)))
  println(Cc(1, 2)(3))
  // Unapplied, it is a function of the primary's parameter.
  val f: Int => V = V.apply
  println(f(5))
  println(List(1, 2).map(V.apply))
  println(List(3).map(V(_)))
  // A placeholder's parameter under an open type parameter is the primary's: no other creator to select.
  println(twice(V(_), 4))
  // A creation reaches every constructor.
  println(new G(1, 2))
  println(new A(1, 2))
  println(new V(1, 2))
  println(new Sub().x)
  println((new V(3, 4) {}).x)
  println(new D.In(1, 2))
  println(new c.In(1, 2))
  println(new S("abc"))
  println(new L(1))
  println(new Df(1))
  // And so does a plain class's proxy.
  println(W(1, 2).x + W(5).x)
