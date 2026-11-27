package a2
class C(val x: Int = 31) { def n: Int = x }
trait T extends C
object Api { @annotation.nowarn("msg=duplicated at each inline site") inline def make(): T = new T {} }
