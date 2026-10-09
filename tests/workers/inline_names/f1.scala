package names

object F1:
  def shows: List[String] =
    two("t1").map(_.show(1)) ++ List(chosen(false, "c1").show(1), plain(false, "p1").show(1), steps(1).toString, capturing(1, 2).show(1), passing(1, 2).value.toString, holding("h1").elem.show(1))
