// teq: --werror
// The pattern definitions scalac 3.8.4 warns of (`Checking.checkIrrefutable`): a pattern whose type
// is narrower than the right side's (`::`, `Some`), with the `.runtimeChecked` fix; a refutable
// extractor (one returning an `Option`); a sequence's rest, checked as `elem*` against the element
// type. A sequence's extractor itself never warns, nor two identical types the type message.
object Ex:
  def unapply(s: String): Option[Int] = if s.nonEmpty then Some(s.length) else None
def f(xs: List[Int], opt: Option[Int]): Unit =
  val List(a) = xs
  val h :: t = xs
  val Some(o) = opt
  val Ex(e) = "eight"
  val List(b, rest*) = xs
  ()
// absent: pattern_binding_refutable.scala:9:
// expect: 10:16: warning: pattern's type ::[Int] is more specialized than the right hand side expression's type List[Int]
// expect: If the narrowing is intentional, this can be communicated by adding `.runtimeChecked` after the expression,
// expect: 11:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 12:7: warning: pattern binding uses refutable extractor `Ex`
// expect: 13:24: warning: pattern's type Int* does not match the right hand side expression's type Int
// absent: List[Int] is more specialized than the right hand side expression's type List[Int]
