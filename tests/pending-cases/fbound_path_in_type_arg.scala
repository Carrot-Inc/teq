// A bound that names its own member through a path inside a type argument is no cycle, as
// under scalac (`type M <: List[t.b.M]`): a member lookup that runs deep through it stays quiet.
object fb:
  object a:
    trait T { t =>
      type M <: List[t.b.M]
      type T <: a.T
      val b: t.T
    }
    lazy val x: a.T = ???
  lazy val m: a.x.M = ???
  trait Show[X]
  given Show[Int] = new Show[Int] {}
  def shown(s: Show[a.x.M]): String = "show"

@main def Main(): Unit =
  println(fb.shown(new fb.Show[fb.a.x.M] {}))
