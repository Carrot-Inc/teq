// A language import of the source version decides as `-source` does (`Feature.sourceVersion`, the
// file's import over the setting): under `import scala.language.future` a refutable pattern
// definition is an error, `.runtimeChecked` still exempting.
import scala.language.`future`
def f(opt: Option[Int]): Unit =
  val Some(o) = opt
  val Some(p) = opt.runtimeChecked
  ()
// expect: 6:17: error: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// absent: pattern_binding_refutable_future_import.scala:7:
