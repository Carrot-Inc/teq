// A member seen through an intersection of two instances of its class, whose invariant parameter
// is then a range of types (`argForParam`'s bounds), approximated as dotty's
// `ApproximatingTypeMap` approximates it at every position: through an alias (`Fun[K]`, a function
// taking the union where the member returns it, the intersection where it takes it), inside an
// invariant class (`Box[List[K]]` a wildcard between the two `List`s), and through a higher-kinded
// parameter of two constructors (`F[Int]` between `List[Int] | Option[Int]` and their meet).
type Fun[T] = T => Unit
class Box[T](val value: T)

trait R[K]:
  def get: Fun[K]
  def put(f: Fun[K]): String
  def boxed: Box[List[K]]
class Impl extends R[Any]:
  def get: Fun[Any] = x => println("got " + x)
  def put(f: Fun[Any]): String = "put " + (f != null)
  def boxed: Box[List[Any]] = Box(List(1, "a"))

trait H[F[_]]:
  def show(x: F[Int]): String
class HImpl extends H[[X] =>> Any]:
  def show(x: Any): String = "shown " + x

def taken(r: R[Int] & R[String]): (Int | String) => Unit = r.get
def takes(r: R[Int] & R[String])(f: (Int & String) => Unit): String = r.put(f)
def nested(r: R[Int] & R[String]): Box[? >: List[Int | String] <: List[Int & String]] = r.boxed
def hk(h: H[List] & H[Option]): String = h.show(List(1)) + ", " + h.show(Some(2))

@main def main(): Unit =
  val r = (new Impl).asInstanceOf[R[Int] & R[String]]
  taken(r)(1)
  taken(r)("s")
  println(takes(r)(x => println("takes " + x)))
  println(nested(r).value.length)
  println(hk((new HImpl).asInstanceOf[H[List] & H[Option]]))
