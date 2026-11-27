package a2
class C[X](val x: Option[X] = None) { def n: Int = x.size + 31 }
trait T extends C[Int]
object Api { inline def mk: T = { class L extends T; new L }; inline def sub: T = { class L extends T; class M extends L; new M } }
