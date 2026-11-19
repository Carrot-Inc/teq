// `==` and `!=` written as a selection are `Any`'s members, found before an extension or a conversion is tried: the
// comparison's own check refuses what cannot be compared, several arguments auto-tupled into the one `Any` parameter.
// scalac refuses each (E172). teq found no member `==` and refused it as no member, or took a conversion to a class
// with an `==` of two parameters and built.
// expect: 16:25: error: Values of types Boolean and String cannot be compared with == or !=
// expect: 17:23: error: Values of types Boolean and (String, String) cannot be compared with == or !=
// expect: 18:23: error: Values of types Int and (Int, Int) cannot be compared with == or !=
// expect: 19:23: error: Values of types K and (String, String) cannot be compared with == or !=
// expect: 20:40: error: Values of types Boolean and (String, String) cannot be compared with == or !=
// expect: 21:38: error: Values of types K and (String, String) cannot be compared with == or !=
// expect: 6 errors found
import scala.language.implicitConversions
class K
class Payload { def ==(x: String, y: String): String = "equal" }
object Dsl { implicit def conv(b: Boolean): Payload = new Payload; implicit def wrap(k: K): Payload = new Payload }
def single(): Boolean = true.==("a")
def pair(): Boolean = true.==("a", "b")
def ints(): Boolean = 1.==(2, 3)
def refs(): Boolean = new K().==("a", "b")
def converted(): Any = { import Dsl.*; true.==("a", "b") }
def wrapped(): Any = { import Dsl.*; new K().==("a", "b") }
