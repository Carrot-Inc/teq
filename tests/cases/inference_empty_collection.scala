@main def run(): Unit =
  val maybe: Option[Set[String]] = None
  val values = maybe.getOrElse(Set.empty)
  println(values + "a")
  println(maybe.getOrElse(Set.empty) + "b")
  println(Map.empty[String, Set[String]].updatedWith("a")(xs => Some(xs.getOrElse(Set.empty) + "b")))
