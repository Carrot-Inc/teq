// A selection the receiver's type lacks is one implicit search in dotty (`tryExtensionOrConversion`, then `inferView`):
// a conversion method (`Candidate.Conversion`, `methodCandidateKind`) and a given with the extension method
// (`Candidate.Extension`, `hasExtMethod`) are candidates of the same contexts, ranked by nesting level, and
// `combineEligibles` hides an outer candidate whose name an inner one has. So the conversion `B.ops` of an inner
// import, whose result has `pick`, beats the outer given `A.ops`'s extension `pick` (and hides it, of one name), and
// `B.other` beats it by level: scalac 3.8.4 prints 2 twice. teq looks up the extension givens apart from the
// conversions (`givens_with_extension`, then `conversions.rs`), so the outer given's extension wins: 1 twice, on
// master too.
import scala.language.implicitConversions
class R
class Wrapper:
  def pick: Int = 2
object A:
  given ops: AnyRef with
    extension (r: R) def pick: Int = 1
object B:
  implicit def ops(r: R): Wrapper = new Wrapper
  implicit def other(r: R): Wrapper = new Wrapper
import A.ops
object Main:
  def main(args: Array[String]): Unit =
    locally {
      import B.ops
      println((new R).pick)
    }
    locally {
      import B.other
      println((new R).pick)
    }
