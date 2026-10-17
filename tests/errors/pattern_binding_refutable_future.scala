// teq: --source future
// Under `-source future` a refutable pattern definition is an error (`report.errorOrMigrationWarning`
// over `MigrationVersion.ForComprehensionUncheckedPathDefs`, an error from `future`), the
// sequence's extractor still silent and `.runtimeChecked` still exempting.
object Ex:
  def unapply(s: String): Option[Int] = if s.nonEmpty then Some(s.length) else None
def f(xs: List[Int], opt: Option[Int]): Unit =
  val List(a) = xs
  val h :: t = xs
  val Some(o) = opt
  val Ex(e) = "eight"
  val Some(p) = opt.runtimeChecked
  ()
// absent: pattern_binding_refutable_future.scala:8:
// expect: 9:16: error: pattern's type ::[Int] is more specialized than the right hand side expression's type List[Int]
// expect: 10:17: error: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 11:7: error: pattern binding uses refutable extractor `Ex`
// absent: pattern_binding_refutable_future.scala:12:
