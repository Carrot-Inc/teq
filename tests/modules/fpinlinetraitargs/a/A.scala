package a2
class C
trait Arg(val v: Int = 11) extends C
object Api {
  @annotation.nowarn("msg=duplicated at each inline site") inline def anon: Arg = new Arg(7) {}
  @annotation.nowarn("msg=duplicated at each inline site") inline def local: Arg = { class L extends Arg(9); new L }
  @annotation.nowarn("msg=duplicated at each inline site") inline def dflt: Arg = new Arg {}
}
