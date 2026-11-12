// A primitive unary operator and a numeric conversion (an Int's unary_+, an Int's toInt)                           
// of a constant val selected through an effectful prefix: TASTy keeps each selection                                 
// (Pickler before FirstTransform, Compiler.scala:50-63; TreePickler.scala:476-508),                                
// which FirstTransform then takes to { pre; 42 }.              
package fix.unaryref

class UrC:
  def h: UrC = this
  val x: 42 = 42
class UrHolder:
  def h: UrHolder = { println("h"); this }
  val plus = +h.x
  val identity = h.x.toInt
  val x: 42 = 42
object UrLib:
  def plus(c: UrC): Int = +c.h.x
  def identity(c: UrC): Int = c.h.x.toInt
  def block(c: UrC): Int = +{ println("pre"); c.h.x }
  def nested(c: UrC): Int = +(+(c.h.x))
