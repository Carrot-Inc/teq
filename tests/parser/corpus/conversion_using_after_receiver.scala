// An implicit conversion with a using clause over its type parameter (doobie's
// `SingleFragment.fromPut[A](a: A)(implicit put: Put[A])`): the receiver fixes `A` before the
// search, so an `Obj` looks for a `Put[Obj]` and does not find `Put[Json]` and `Put[Obj]`
// ambiguous for an `A >: Obj`.
import scala.language.implicitConversions

final class Put[A](val show: A => String)
final case class SF[+A](fr: String)
object SF:
  implicit def fromPut[A](a: A)(implicit put: Put[A]): SF[A] = SF(put.show(a))

def sql(a: SF[?]*): String = a.map(_.fr).mkString(",")
def one(a: SF[?]): String = a.fr

class Json
class Obj extends Json
given Put[Json] = Put(_ => "json")
given Put[Obj] = Put(_ => "obj")
given Put[Int] = Put(i => "int " + i)

@main def main(): Unit =
  val o = Obj()
  println(one(o))
  println(sql(o, 3))
  val s: SF[?] = new Json
  println(s.fr)
