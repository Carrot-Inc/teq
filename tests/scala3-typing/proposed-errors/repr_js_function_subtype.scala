// expect: 9:51: error: Cannot prove that Int => Int <:< scala.scalajs.js.Function1[Int, Int].
// The design pass's JsSubtype probe: `js.Function1` is a trait, not Scala's function type (Scala.js 1.22.0's
// Function.scala 113-115; Definitions.scala 1851-1861 excludes it from the function types), so no Scala function
// conforms to it; scalac refuses (E172), teq's std aliases the trait and accepts. Moves with type-representation's
// landing 6.
import scala.scalajs.js
object JsSubtype:
  val ev =
    summon[(Int => Int) <:< js.Function1[Int, Int]]
