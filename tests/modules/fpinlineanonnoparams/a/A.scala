package a2
class C { def n: Int = 31 }
trait T extends C
object Api { @annotation.nowarn("msg=duplicated at each inline site") inline def make(): T = new T {} }
