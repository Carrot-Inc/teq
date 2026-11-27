package a2
class C[X](val x: Option[X] = None) { def n: Int = x.size + 31 }
class W extends C[Int]
class V[Y] extends C[Y]
trait T extends C[Int]
class K extends T
object U { def w: Int = { class L extends C[Int]; new L().n }; def k: Int = { class L extends T; new L().n } }
