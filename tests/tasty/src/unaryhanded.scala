// A primitive unary operator or numeric conversion (an Int's unary_+, a Long's toLong) whose                      
// operand is itself a primitive unary operation or conversion: TASTy keeps both selections                           
// (Pickler before FirstTransform, Compiler.scala:50-63; TreePickler.scala:476-508), each                               
// of the constant type ConstFold gives it (ConstFold.scala:49-54), which FirstTransform                  
// then takes to { pre; v }.             
package fix.unaryhanded

class UhC:
  def h: UhC = this
  val x: 42 = 42
class UhHolder:
  def h: UhHolder = { println("h"); this }
  val plus = +(-h.x)
  val identity = h.x.toLong.toLong
  val x: 42 = 42
object UhLib:
  def plus(c: UhC): Int = +(-c.h.x)
  def identity(c: UhC): Long = c.h.x.toLong.toLong
  def twice(c: UhC): Int = +(+(-c.h.x))
  def negated(c: UhC): Long = (-c.h.x).toLong.toLong
  def inverted(c: UhC): Int = +(~c.h.x)
  def pure(c: UhC): Int = +(-c.x)
