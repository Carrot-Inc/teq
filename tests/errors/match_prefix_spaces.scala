// teq: --werror
// The spaces keep a class's prefix below the top of a match: a sealed trait nested in a class splits into
// its children through the same prefix (dotty's `TypeOps.childPrefix`), so `Some(a.I(n))` covers an
// `Option[a.T]` and `Some(b.I(n))` does not; a counterexample through a prefix is shown with it (`a.I(_)`);
// a wildcard after cases that leave only `null` of a nullable scrutinee is E121. Master warned of `same`
// and named `I(_)`, and gave a plain unreachable case for `nullable`.
class O:
  sealed trait T
  case class I(n: Int) extends T
val a = new O
val b = new O
def same(x: Option[a.T]): Int = x match { case Some(a.I(n)) => n; case None => 0 }
def cross(x: Option[a.T]): Int = x match { case Some(b.I(n)) => n; case None => 0 }
def top(x: a.I): Int = x match { case b.I(n) => n }
def nullable(x: (Int, String)): Int = x match
  case (n, s) => n
  case _ => 0
enum E { case A, B }
def missing(x: Option[E]): Int = x match
  case Some(E.A) => 1
  case None => 0
// expect: 13:34: warning: match may not be exhaustive.
// expect: It would fail on pattern case: Some(_)
// expect: 14:24: warning: match may not be exhaustive.
// expect: It would fail on pattern case: a.I(_)
// expect: 17:8: warning: Unreachable case except for null (if this is intentional, consider writing case null => instead).
// expect: 19:34: warning: match may not be exhaustive.
// expect: It would fail on pattern case: Some(B)
// absent: 12:33: warning
