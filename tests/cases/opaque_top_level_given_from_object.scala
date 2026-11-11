// A top-level opaque type named in the types of members of objects of its file, read from another object of
// the file. Outside the file's top-level definitions the type is one opaque reference whoever names it
// (dotty: `TypeRef(TermRef(p, <file>$package), T)`): a given of its companion or of an imported object is found
// for it, also under a type constructor and for a generic opaque type, and a top-level definition's inferred
// type conforms to it.
trait Show[A]:
  def show(a: A): String

opaque type Id = Int
object Id:
  def apply(n: Int): Id = n
  given Show[Id] = new Show[Id] { def show(a: Id): String = "id" }
  given listShow: [A] => (s: Show[A]) => Show[List[A]] = new Show[List[A]] { def show(as: List[A]): String = as.map(s.show).mkString(",") }

opaque type Tagged[A] <: A = A
object Tagged:
  def apply[A](a: A): Tagged[A] = a
  given tagged: [A] => Show[Tagged[A]] = new Show[Tagged[A]] { def show(a: Tagged[A]): String = "tagged" }

object Other:
  given Show[Option[Id]] = new Show[Option[Id]] { def show(o: Option[Id]): String = "opt" }

val top = Id(1)
def same(a: Id, b: Id): Boolean = a == b

object Use:
  val u: Id = top
  val l: List[Id] = List(top)
  def run(): Unit =
    println(summon[Show[Id]].show(Id(1)))
    println(summon[Show[List[Id]]].show(List(Id(1), Id(2))))
    println(summon[Show[Tagged[String]]].show(Tagged("a")))
    import Other.given
    println(summon[Show[Option[Id]]].show(None))
    println(same(u, top))
    println(l.size)

@main def run(): Unit = Use.run()
