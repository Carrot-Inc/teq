package a2
class C[X](val x: Option[X] = None) { def n: Int = x.size + 31 }
trait T extends C[Int]
object O extends T
