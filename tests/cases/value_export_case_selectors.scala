// A wildcard export through a value leaves out the case class's selectors `_1`..`_n`, which the
// exporting case class has of its own (dotty's addWildcardForwarders excludes synthetic members).
case class User(id: String, name: String):
  def title: String = name.toUpperCase

case class WithPerfs(user: User, perfs: Int):
  export user.*

@main def run(): Unit =
  val w = WithPerfs(User("u1", "ann"), 3)
  println(w.id)
  println(w.title)
  println(w._1)
  println(w._2)
  val WithPerfs(u, p) = w
  println(s"$u $p")
