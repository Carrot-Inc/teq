package eua

// An `if` without `else`, whose pickle holds the `()` scalac puts in its place, beside an explicit
// `else ()` whose value is the branch's or the unit.
object Steps:
  var log: List[String] = Nil
  def note(s: String): Unit = if s.nonEmpty then log = s :: log
  def pick(c: Boolean): Any = if c then 1 else ()
  def last(c: Boolean): Any =
    val v = if c then "kept" else ()
    v
