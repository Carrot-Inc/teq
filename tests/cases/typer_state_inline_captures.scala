import scala.compiletime.{requireConst, summonInline}

// A plain inline call whose expansion names a local (`summonInline` finding a local given) inside
// a lambda made into a class for a SAM type, a local method, a function, a local class and an
// anonymous class: each captures what the expansion names, the SAM class's captures read after
// its calls are expanded. `requireConst` reads a pending call's
// expansion.
trait F:
  def apply(): Int

trait K:
  def k: Int

object M:
  inline def get: Int = summonInline[Int]
  inline def one: Int = 1

@main def run(): Unit =
  requireConst(M.one)
  given n: Int = 42
  val sam: F = () => M.get
  def f(): Int = M.get
  val g = () => M.get
  class L:
    def h: Int = M.get
  val o = new K { def k: Int = M.get }
  println(sam())
  println(f() + g() + new L().h + o.k)
