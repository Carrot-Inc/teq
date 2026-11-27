package a2
class C(val x: Int = 31) { def n: Int = x }
trait T extends C
object Api { inline def mk: T = { class L extends T; new L }; inline def sub: T = { class L extends T; class M extends L; new M } }
