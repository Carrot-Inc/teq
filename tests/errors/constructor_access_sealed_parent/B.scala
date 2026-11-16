// expect: B.scala:12:12: error: Cannot extend sealed class Sea in a different source file
// expect: B.scala:13:12: error: Cannot extend sealed class SeaP in a different source file
// expect: B.scala:14:16: error: constructor SG cannot be accessed as a member of SG from object Use
// expect: B.scala:15:18: error: constructor SG cannot be accessed as a member of SG from class C2
// expect: B.scala:16:19: error: constructor SG0 cannot be accessed as a member of SG0 from class C2b
// expect: B.scala:17:18: error: constructor SG0 cannot be accessed as a member of SG0 from trait T3
// expect: B.scala:18:18: error: Cannot extend sealed class SD in a different source file
// expect: 7 errors found
package app
import lib.*
object Use:
  val c1 = new Sea(1) {}
  val c2 = new SeaP(1) {}
  val s2 = new SG(1) { def y = 1 }
class C2 extends lib.SG(1)
class C2b extends lib.SG0
trait T3 extends lib.SG0
class C3 extends lib.SD
