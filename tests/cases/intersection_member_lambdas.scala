// The last fold's shapes, as scalac 3.8.4 types them: a polymorphic member whose bound is the
// parameter two instances make a range (`[T <: K]` read as `[T <: Int | String]`,
// `derivedLambdaType`), a higher-kinded parameter and its argument both ranges (`F[K]` of
// `R[List, Int] & R[Option, String]` read as `List[..] & Option[..]`, `derivedAppliedType`; typed
// alone, as no value is both), receivers that are applied parameters bounded by an intersection
// (`F[X] <: R["k"] & R["v"]`, the instance met through the bound as `baseTypeOf` meets it), and a
// result nested 34 lists deep through aliases, approximated whole.
trait P[K]:
  def get: [T <: K] => T => T
class PS extends P[Any]:
  def get: [T <: Any] => T => T = [T] => (x: T) => x

trait H[F[_], K]:
  def get: F[K]
def both(h: H[List, Int] & H[Option, String]): List[Int & String] & Option[Int & String] = h.get

trait R[K <: String]:
  def get(k: K): Any
  def get(n: Int): Int
class Store extends R[String]:
  def get(k: String): Any = "got " + k
  def get(n: Int): Int = n + 1

trait S[K <: String]:
  def one(k: K): String
class SS extends S[String]:
  def one(k: String): String = "one " + k

type L[T] = List[T]
type L2[T] = L[L[T]]
type L3[T] = L2[L2[T]]
type L4[T] = L3[L3[T]]
type L5[T] = L4[L4[T]]
type L6[T] = L5[L5[T]]
trait D[K]:
  def get: L6[K]
class DS extends D[Any]:
  def get: L6[Any] = Nil

def poly(p: P[Int] & P[String]): [T <: Int | String] => T => T = p.get
def overloaded[F[X] <: R["k"] & R["v"]](r: F[Int]): String = r.get("v").toString + ", " + r.get(1)
def plain[F[X <: String] <: S[X] & S["v"]](s: F["k"]): String = s.one("v") + ", " + s.one("k")
def deep(d: D[Int] & D[String]): L6[Int & String] = d.get

@main def main(): Unit =
  val f = poly((new PS).asInstanceOf[P[Int] & P[String]])
  println(f[Int](3) + f[String](4.toString).length)
  println(overloaded[[X] =>> R["k"] & R["v"]]((new Store).asInstanceOf[R["k"] & R["v"]]))
  println(plain[[X <: String] =>> S[X] & S["v"]]((new SS).asInstanceOf[S["k"] & S["v"]]))
  println(deep((new DS).asInstanceOf[D[Int] & D[String]]).length)
