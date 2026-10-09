// The controls of broken-array: a ClassTag of Nothing written, bound by an argument, or of an
// argument's own type (`throw`), beside an error inside an argument that leaves the argument's type
// as it is, which scalac reports as well.
object Controls:
  val written = Array.apply[Nothing]({ val bad: Int = "s"; throw new Exception() })
  val bound = Array(1).flatMap(x => { val bad: Int = "s"; List.empty[Nothing] })
  val beside = Array(1).flatMap(x => { nope; List.empty[Nothing] })
  val filled = Array.fill({ val bad: Int = "s"; 1 })(throw new Exception())
  val thrown = Array(1).flatMap(x => { val bad: Int = "s"; throw new Exception() })
