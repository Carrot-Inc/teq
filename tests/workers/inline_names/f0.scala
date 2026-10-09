package names

object F0:
  def shows: List[String] =
    two("t0").map(_.show(0)) ++ List(chosen(false, "c0").show(0), plain(false, "p0").show(0), steps(0).toString, capturing(0, 1).show(0), passing(0, 1).value.toString, holding("h0").elem.show(0))
