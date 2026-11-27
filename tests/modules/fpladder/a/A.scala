package a2
class Mono(using val v: ValueOf[31]) { def n: Int = v.value }
trait MT extends Mono
class MK extends MT
object MO extends MT
class Generic[A](using val v: ValueOf[A]) { def n: A = v.value }
trait GT extends Generic[31]
class GK extends GT
object GO extends GT
class Plain(val k: Int = 7) { def n: Int = k }
trait PT extends Plain
class PK extends PT
object PO extends PT
object Api { def mono: Int = { class L extends MT; new L().n } }
