// lila: report/ReportApi.scala:583 (`Accuracy { Math.round((nbEngines + 0.5f) / (userIds.length + 2f) * 100) }`)
// java.lang.Math.round is overloaded: round(float): int and round(double): long; a Float
// argument selects the Int one.
@main def run() =
  val nbEngines = 1
  val userIds = List("a", "b")
  val accuracy: Int = Math.round((nbEngines + 0.5f) / (userIds.length + 2f) * 100)
  println(accuracy)
