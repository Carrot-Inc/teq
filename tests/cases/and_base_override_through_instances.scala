// An override of a member whose owner a trait's parents give at two instances (`BothE`'s `Ex[Int] & Ex[String]`):
// dotty's override check reads the overridden `unapply` seen from `BothE`, `(x: Any): Option[Int & String]` (RefChecks'
// `self.memberInfo(other)`, `argForParam`'s `AndType` arm and `AsSeenFromMap.derivedSelect`'s range at the result's
// variance), which the override matches:
// scalac 3.8.4 accepts the program,
// and teq is to accept it too.
trait Ex[A]:
  def unapply(x: Any): Option[A]
trait IE extends Ex[Int]
trait SE extends Ex[String]
trait BothE extends IE with SE:
  def unapply(x: Any): Option[Int & String] = None
@main def run(): Unit = println("ok")
