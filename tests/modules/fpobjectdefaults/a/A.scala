package a2
class C(val x: Int = 31) { def n: Int = x }
trait T extends C
object O extends T
