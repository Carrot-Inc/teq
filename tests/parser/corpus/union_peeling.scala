// Members of a union parameter type without type variables take what conforms to them first;
// what is left binds the type variable: `Entry | Unit` against `A | Unit` gives `A = Entry`.
final case class Entry(name: String)
final case class Other(n: Int)
object Missing
type Nullable[A] = A | Missing.type

def missingToOption[A](a: A | Missing.type): Option[A] = a match
  case Missing => None
  case a => Some(a.asInstanceOf[A])

def keep[A](x: A | Other): List[A] = x match
  case _: Other => Nil
  case a => List(a.asInstanceOf[A])

def opt[A](x: Option[A] | Unit): Option[A] = x match
  case _: Unit => None
  case o => o.asInstanceOf[Option[A]]

def two[A](x: A | Unit, y: A | Unit): List[A] = List(x, y).flatMap {
  case _: Unit => Nil
  case a => List(a.asInstanceOf[A])
}

def nested[A, B](x: (A | Unit) | (B | Boolean), b: B): (List[A], B) = (x match
  case _: Unit => Nil
  case _: Boolean => Nil
  case a => List(a.asInstanceOf[A])
) -> b

extension [A](x: A | Unit) def toOpt: Option[A] = x match
  case _: Unit => None
  case a => Some(a.asInstanceOf[A])

@main def main(): Unit =
  val e: Entry | Unit = Entry("n")
  println(e.toOpt.map(_.name))
  val u: Entry | Unit = ()
  println(u.toOpt.map(_.name))
  val k2: Entry | String | Other = "s"
  println(keep(k2).map(_.toString))
  val n: Nullable[Entry] = Entry("n")
  println(missingToOption(n).map(_.name))
  val m: Nullable[Entry] = Missing
  println(missingToOption(m).map(_.name))
  val o: Option[Entry] | Unit = Some(Entry("o"))
  println(opt(o).map(_.name))
  val e1: Entry | Unit = Entry("1")
  val e2: Entry | Unit = ()
  println(two(e1, e2).map(_.name))
  val t: Boolean | Entry | Unit = true
  println(nested(t, "s"))
  val en: Boolean | Entry | Unit = Entry("e")
  println(nested(en, "s")._1.map(_.name))
  val num: Int | Unit = 3
  println(num.toOpt.map(_ + 1))
