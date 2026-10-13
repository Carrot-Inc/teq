package ucpa
trait Parent(val n: Int)
trait Ctx(using val m: Int)
class C(using k: Int) extends Parent(k):
  def doubled = k * 2
class D(k: Int) extends Ctx(using k)
class E(using k: Int) extends Parent(k), Ctx
class F(using k: Int) extends Ctx
class G(using k: Int) extends Ctx(using k)
object P:
  def make(using k: Int): C = new C
  given withCtx(using k: Int): Ctx with
    def twice = m + k
