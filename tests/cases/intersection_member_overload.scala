// One member through two instances of its class is one member of the intersection, seen from the
// whole receiver (dotty's `Types.findMember`: a class part's `ClassDenotation.findMember` with the
// receiver as `pre`, whose base type of an intersection meets the two instances, and
// `NamedType.argForParam` of that base, an invariant parameter's bounds the union of the arguments
// to the intersection of them): `R["k"] & R["v"]`'s `get` takes `"k" | "v"` in either order, a
// polymorphic and a curried member alike; a result of the parameter's type is their intersection.
// Each part refining `get` keeps the class member's `Any` beside the refinements.
import scala.reflect.Selectable.reflectiveSelectable

trait R[K <: String]:
  def get(k: K): Any
  def poly[T](k: K, x: T): T
  def curried(n: Int)(k: K): Any
  def same(k: K): K
class Store(tag: String) extends R[String]:
  def get(k: String): Any = tag + ":" + k
  def poly[T](k: String, x: T): T = x
  def curried(n: Int)(k: String): Any = k * n
  def same(k: String): String = k + k

type Rec[K <: String, V] = R[K] { def get(k: K): V }

trait G[A]:
  def get: A
class GS extends G[String]:
  def get: String = "gs"

def r2: R["k"] & R["v"] = Store("s").asInstanceOf[R["k"] & R["v"]]
def res1: Any = r2.get("k")
def res2: Any = r2.get("v")
def reversed(r: R["v"] & R["k"]): Any = r.get("k")
def three(r: R["k"] & R["v"] & R["z"]): Any = r.get("z")
def polyCall(r: R["k"] & R["v"]): Int = r.poly[Int]("k", 1) + r.poly("v", 2)
def curriedCall(r: R["k"] & R["v"]): Any = r.curried(2)("v")
def eta(r: R["k"] & R["v"]): ("k" | "v") => Any = r.get
def refined(r: Rec["k", String] & Rec["v", Int]): Any = r.get("k")
def widened(x: G[Any] & G[String]): String = x.get

@main def main(): Unit =
  println(res1)
  println(res2)
  println(reversed(Store("r").asInstanceOf[R["v"] & R["k"]]))
  println(three(Store("t").asInstanceOf[R["k"] & R["v"] & R["z"]]))
  println(polyCall(r2))
  println(curriedCall(r2))
  println(eta(r2)("v"))
  println(refined(Store("x").asInstanceOf[Rec["k", String] & Rec["v", Int]]))
  println(widened((new GS).asInstanceOf[G[Any] & G[String]]).length)
