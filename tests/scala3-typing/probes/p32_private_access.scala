class Vault:
  private val secret = 1
  protected val semi = 2
  private[this] val own = 3
  def peek(other: Vault) = other.secret + other.own
object Vault:
  def open(v: Vault) = v.secret
trait Base:
  protected def guarded: Int = 1
class Impl extends Base:
  def get = guarded
@main def run(): Unit =
  val v = Vault()
  println(v.semi)
  println(Impl().guarded)
