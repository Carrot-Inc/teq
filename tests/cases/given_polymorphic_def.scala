// A given with type parameters is a def, as one with a using clause is (dotty's
// `Parsers.givenDef`): an alias's right-hand side runs at each summon and a structural given's
// class is made at each, on an object, at the top level and in a class; one without parameters is
// a lazy val, made once.
class Box[A](val tag: Int)
trait Ord[A] { def id: Int }

object Api:
  var n = 0
  given box[A]: Box[A] = { n += 1; new Box[A](n) }
  given ordList[A]: Ord[List[A]] with
    n += 10
    def id = n
  given once: Ord[Char] = { n += 100; new Ord[Char] { def id = n } }
  def count: Int = { summon[Box[Int]]; summon[Box[String]]; n }

var top = 0
given topBox[A]: Ord[Option[A]] = { top += 1; new Ord[Option[A]] { def id = top } }

class Holder:
  var k = 0
  given inner[A]: Box[A] = { k += 1; new Box[A](k * 100) }
  def use = summon[Box[Int]].tag + summon[Box[String]].tag

@main def run(): Unit =
  import Api.given
  println(Api.count)
  val a = summon[Box[Int]]
  val b = summon[Box[Int]]
  println(s"${a.tag} ${b.tag} ${a eq b}")
  val o1 = summon[Ord[List[Int]]]
  val o2 = summon[Ord[List[Int]]]
  println(s"${o1.id} ${o2.id} ${o1 eq o2} ${Api.n}")
  println(s"${summon[Ord[Char]] eq summon[Ord[Char]]} ${Api.n}")
  println(s"${summon[Ord[Option[Int]]].id} ${summon[Ord[Option[String]]].id} $top")
  val h = new Holder
  println(s"${h.use} ${h.k}")
  def poly[A](using b: Box[A]) = b.tag
  println(s"${poly[Int] + poly[String]} ${Api.n}")
