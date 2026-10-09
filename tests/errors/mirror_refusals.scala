// expect: 17:32: error: No given instance of type Mirror.Of[Open] was found.
// expect: Failed to synthesize an instance of type Mirror.Of[Open]:
// expect: * trait Open is not a generic product because it is not a case class
// expect: * trait Open is not a generic sum because it is not a sealed trait
// expect: 18:40: error: No given instance of type Mirror.ProductOf[Plain] was found.
// expect: * class Plain is not a generic product because it is not a case class
// expect: 19:36: error: No given instance of type Mirror.SumOf[Plain] was found.
// expect: * class Plain is not a generic sum because it is not a sealed class
// expect: 20:34: error: No given instance of type Mirror.Of[Locked] was found.
// expect: * class Locked is not a generic product because the constructor of class Locked is inaccessible from the calling scope.
// expect: 21:31: error: No given instance of type Mirror.Of[Int] was found.
import scala.deriving.Mirror
trait Open
class Plain(x: Int)
case class Locked private (x: Int)
case object Locked
def a = summon[Mirror.Of[Open]]
def b = summon[Mirror.ProductOf[Plain]]
def c = summon[Mirror.SumOf[Plain]]
def d = summon[Mirror.Of[Locked]]
def e = summon[Mirror.Of[Int]]
