// A tuple literal argument whose formal is a type variable bounded by the expected type
// (lila's `val reportAngles: List[(Angle, String)] = List(("skills", "Skills"), ...)`).
type Angle = "skills" | "opening"

@main def run() =
  val reportAngles: List[(Angle, String)] = List(
    ("skills", "Skills"),
    ("opening", "Openings")
  )
  println(reportAngles)
