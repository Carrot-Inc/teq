package pg

// A given with type parameters is a def (dotty's `Parsers.givenDef`): an alias's right-hand side
// runs at each summon, a structural one's class is made at each; on an object, at the top level
// and in a class, read over the products as in the whole build.

class Box[A](val tag: Int)
trait Ord[A] { def id: Int }

object Api:
  var n = 0
  given box[A]: Box[A] = { n += 1; new Box[A](n) }
  given ordList[A]: Ord[List[A]] with
    n += 10
    def id = n
  def count: Int = { summon[Box[Int]]; summon[Box[String]]; n }

var top = 0
given topBox[A]: Ord[Option[A]] = { top += 1; new Ord[Option[A]] { def id = top } }

class Holder:
  var k = 0
  given inner[A]: Box[A] = { k += 1; new Box[A](k * 100) }
  def use = summon[Box[Int]].tag + summon[Box[String]].tag
