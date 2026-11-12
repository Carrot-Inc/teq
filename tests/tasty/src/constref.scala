// A constant val's selection on an ordinary value and an identifier of an object's constant member: FirstTransform
// takes them to their literals after the pickle's phase (Pickler before FirstTransform), so TASTy keeps the
// references (`c.x`, `y`).                                                              
package fix.constref

class CrHolder:
  val x: 42 = 42
object CrRead:
  val y: 43 = 43
  def read(c: CrHolder): Int = c.x
  def readY: Int = y
