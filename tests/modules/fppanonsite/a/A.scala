package anon_site
object Count { var n = 0; def next(): Int = { n += 1; n } }
class Base(val x: Int = Count.next())
trait T extends Base
object Api { @annotation.nowarn("msg=duplicated at each inline site") inline def mk(): T = new T {} }
