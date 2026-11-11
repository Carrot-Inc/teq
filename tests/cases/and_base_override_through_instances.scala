// An override of a member whose owner a trait's parents give at two instances (`BothE`'s `Ex[Int] & Ex[String]`):
// dotty's override check reads the overridden `unapply` seen from `BothE`, `(x: Any): Option[Int & String]` (RefChecks'
// `self.memberInfo(other)`, `argForParam`'s `AndType` arm and `AsSeenFromMap.derivedSelect`'s range at the result's
// variance; the closure adversary's probe SRC-AF-bothe-unapply-seen, pass on 5bd85bcd1, round 23, and lane Q's
// RC-bothe-unapply, round 24), which the override matches: scalac 3.8.4 accepts the program, and so does teq, whose
// override check reads the owner's parameters through the same ranges (`sig_seen_from_owner`, src/typer/subtype.rs).
trait Ex[A]:
  def unapply(x: Any): Option[A]
trait IE extends Ex[Int]
trait SE extends Ex[String]
trait BothE extends IE with SE:
  def unapply(x: Any): Option[Int & String] = None
@main def run(): Unit = println("ok")
