// A nested search that succeeds inside a candidate that fails uses nothing.
trait C
object C { given fallback: C = new C {} }
object G {
  given aux: Int = 1
  given failed(using Int, String): C = new C {}
}
object Use {
  import G.{aux, failed}
  val result: C = summon[C]
}
