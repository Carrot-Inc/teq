// Under strictEquality a literal or stable identifier pattern is compared by `==`, whose CanEqual it asks
// (dotty's checkEqualityEvidence): an enum without derives CanEqual, an object against Any, in alternatives,
// a tuple's element and a union with Null.
import scala.language.strictEquality
enum Color:
  case Red, Green
enum Shape derives CanEqual:
  case Sq, Ci
case object Solo
object Ok { given CanEqual[Solo.type, Solo.type] = CanEqual.derived }
class Box(val n: Int)
object Main:
  val Pi = 3
  val Name = "n"
  def e1(c: Color) = c match { case Color.Red => 1; case Color.Green => 2 }
  def e2(s: Shape) = s match { case Shape.Sq => 1; case Shape.Ci => 2 }
  def e3(n: Int) = n match { case 1 => 1; case Pi => 2; case _ => 3 }
  def e4(s: String) = s match { case "a" => 1; case Name => 2; case _ => 3 }
  def e5(o: Option[Int]) = o match { case None => 1; case Some(_) => 2 }
  def e6(l: List[Int]) = l match { case Nil => 1; case _ => 2 }
  def e7(a: Any) = a match { case Solo => 1; case _ => 2 }
  def e8(s: Solo.type) = s match { case Solo => 1 }
  def e9(c: Color) = c match { case Color.Red | Color.Green => 1 }
  def e10(t: (Color, Int)) = t match { case (Color.Red, 1) => 1; case _ => 2 }
  def e11(b: Box) = b match { case null => 1; case _ => 2 }
  def e12(l: Long) = l match { case 1 => 1; case _ => 2 }
  def e13(c: Char) = c match { case 'a' => 1; case 97 => 2; case _ => 3 }
  def e14(c: Color | Null) = c match { case null => 0; case Color.Red => 1; case _ => 2 }

// expect: pattern_can_equal_strict.scala:15:37: error: values of types Color and Color cannot be compared with == or !=
// expect: pattern_can_equal_strict.scala:15:58: error: values of types Color and Color cannot be compared with == or !=
// expect: pattern_can_equal_strict.scala:21:35: error: values of types Solo and Any cannot be compared with == or !=
// expect: pattern_can_equal_strict.scala:23:37: error: values of types Color and Color cannot be compared with == or !=
// expect: pattern_can_equal_strict.scala:23:49: error: values of types Color and Color cannot be compared with == or !=
// expect: pattern_can_equal_strict.scala:24:46: error: values of types Color and Color cannot be compared with == or !=
// expect: pattern_can_equal_strict.scala:28:61: error: values of types Color and Color | Null cannot be compared with == or !=
