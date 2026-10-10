package constantproducts

// A constant `final val`'s right side that is no literal is the constructor's work through the
// products as in one program (dotty's `Constructors`): a trait's, a class's and an object's run
// once, where each stands; the getter answers the constant (`Memoize`).
trait T:
  final val K: 7 = { println("trait-init"); 7 }
class C extends T:
  final val L: 9 = { println("class-init"); 9 }
  final val W: Int = 8
object O:
  final val K: 11 = { println("object-init"); 11 }
  def touch: Int = 0
