import pg.*
import pg.given

@main def run(): Unit =
  import Api.given
  println(Api.count)
  val a = summon[Box[Int]]
  val b = summon[Box[Int]]
  println(s"${a.tag} ${b.tag} ${a eq b}")
  val o1 = summon[Ord[List[Int]]]
  val o2 = summon[Ord[List[Int]]]
  println(s"${o1.id} ${o2.id} ${o1 eq o2} ${Api.n}")
  println(s"${summon[Ord[Option[Int]]].id} ${summon[Ord[Option[String]]].id} $top")
  val h = new Holder
  println(s"${h.use} ${h.k}")
  def poly[A](using b: Box[A]) = b.tag
  println(poly[Int] + poly[String])
