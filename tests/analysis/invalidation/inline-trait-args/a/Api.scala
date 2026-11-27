package inl
class Base
trait Arg(val x: Int = 11) extends Base
object Api { @annotation.nowarn("msg=duplicated at each inline site") inline def mk(): Arg = new Arg(7) {} }
