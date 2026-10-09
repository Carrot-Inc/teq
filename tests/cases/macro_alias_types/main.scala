import optics.*
import Macros.*

case class P(name: String, age: Int)

@main def run(): Unit =
  val nameLens: Lens[P, String] = Lens[P, String](_.name)(n => p => p.copy(name = n))
  val ageLens = Lens[P, Int](_.age)(a => p => p.copy(age = a))
  println(describe[Lens[P, String]])
  println(describe[PLens[P, P, String, String]])
  println(describe[PLens[P, Int, String, String]])
  println(describe[Holder.Optional[P, Int]])
  println(describe[List[Lens[P, Int]]])
  println(describeTerm(Lens[P, Int](_.age)(a => p => p.copy(age = a))))
  println(describeTerm(nameLens))
  println(describeTerm(ageLens))
  println(describeTerm(nameLens.andThen(Lens[String, Int](_.length)(_ => s => s))))
  println(describeTerm(Holder.opt[P, Int](p => Some(p.age))))
  println(describeMember[Lens.type]("apply"))
  println(describeMember[Holder.type]("opt"))
  println(relate[Lens[P, String], PLens[P, P, String, String]])
  println(relate[PLens[P, P, String, String], Lens[P, String]])
  println(relate[Lens[P, String], Lens[P, Int]])
  println(relate[Holder.Optional[P, Int], PLens[P, P, Option[Int], Option[Int]]])
  println(nameLens.get(P("a", 1)) + " " + ageLens.set(5)(P("a", 1)))
