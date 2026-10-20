// lila: fishnet/JsonApi.scala:104 (`def analysisFromWork(m: Work.Analysis): Analysis`)
// The prefix of a type selection `Work.Analysis` is a term: an inner type-only `Work` (a trait)
// does not hide the outer object `Work`.
object Work:
  case class Analysis(id: String)

object JsonApi:
  sealed trait Work
  case class Analysis(id: Int) extends Work
  def analysisFromWork(m: Work.Analysis): Analysis = Analysis(m.id.length)

@main def run() = println(JsonApi.analysisFromWork(Work.Analysis("abc")))
