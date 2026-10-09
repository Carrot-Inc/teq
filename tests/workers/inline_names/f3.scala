package names

object F3:
  def shows: List[String] =
    two("t3").map(_.show(3)) ++ List(chosen(false, "c3").show(3), plain(false, "p3").show(3), steps(3).toString, capturing(3, 4).show(3), passing(3, 4).value.toString, holding("h3").elem.show(3))
