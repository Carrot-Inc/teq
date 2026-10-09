package names

object F2:
  def shows: List[String] =
    two("t2").map(_.show(2)) ++ List(chosen(false, "c2").show(2), plain(false, "p2").show(2), steps(2).toString, capturing(2, 3).show(2), passing(2, 3).value.toString, holding("h2").elem.show(2))
