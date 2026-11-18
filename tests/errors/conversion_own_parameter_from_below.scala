// A conversion whose result is its own type parameter, held covariantly and bounded only above, is instantiated from
// below as dotty's `instantiateTypeVars` takes it (Inferencing.scala 815: `fromBelow = v == 1 || ...`): to `Nothing`,
// which has no member of the bound, so the selection that asked for the conversion is refused, through an object's
// named import, a value's wildcard, a local def and a trait's bound alike. scalac refuses each; teq took the bound.
// expect: 17:43: error: value shout is not a member of Nothing
// expect: 18:46: error: value shout is not a member of Nothing
// expect: 19:106: error: value shout is not a member of Nothing
// expect: 20:47: error: value shout is not a member of Nothing
// expect: 4 errors found
import scala.language.implicitConversions
class Rich(val s: String) { def shout: String = s }
trait Shout { def shout: String }
class Loud(val s: String) extends Shout { def shout: String = s }
object Good { implicit def conv[T <: Rich](b: Boolean): T = new Rich("good").asInstanceOf[T] }
object Bounds { implicit def conv[T <: Shout](b: Boolean): T = new Loud("bounds").asInstanceOf[T] }
class Dsl { implicit def conv[T <: Rich](b: Boolean): T = new Rich("dsl").asInstanceOf[T] }
def named(): String = { import Good.conv; true.shout }
def wildcard(d: Dsl): String = { import d.*; true.shout }
def local(): String = { implicit def conv[T <: Rich](b: Boolean): T = new Rich("local").asInstanceOf[T]; true.shout }
def bounded(): String = { import Bounds.conv; true.shout }
