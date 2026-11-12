// Local lazy vals over their holders (dotty's transformLocalDef: a LazyInt ... LazyRef allocated at the head of its
// block by transformStats, read through an accessor, computed by an initialiser under the holder's monitor), as the
// code around them sees them (a local def reading one before its definition: tests/cases/lazy_local_forward): a
// lambda (of a primitive one), a local class (an anonymous class whose initialiser reads the enclosing instance:
// tests/cases/lazy_local_enclosing_this), a local trait whose member reads one whose initialiser captures
// another local, a local def and a lazy val that read each other, a var the initialiser reads and writes, an initialiser that makes a local class whose method
// calls an enclosing local def (beside the same body as a local def), a local case class holding one (and its copy),
// and a local object.
object Main:
  def lambdaOfPrimitive(): Int =
    var runs = 0
    lazy val p: Int = { runs += 1; 20 }
    val f = (k: Int) => p + k
    f(1) + f(2) + runs

  def localClass(): String =
    val base = "base"
    lazy val l: String = base + " of a local class"
    class Reader { def read: String = l }
    new Reader().read + " / " + new Reader().read

  def localTrait(): String =
    val other = "other local"
    lazy val l: String = "trait read " + other
    trait Reads { def get: String = l }
    class Impl extends Reads
    new Impl().get

  def mutual(n: Int): Int =
    def even(k: Int): Boolean = if k == 0 then true else odd(k - 1)
    lazy val odd: Int => Boolean = k => if k == 0 then false else even(k - 1)
    if even(n) then 1 else 0

  def varInInitialiser(): String =
    var n = 1
    lazy val x: Int = { n += 1; n }
    val first = x
    n = 10
    s"$first $n ${x + n}"

  def makesLocalClass(): String =
    def helper(k: Int): String = "helper " + k
    lazy val viaLazy: String = { class K { def go: String = helper(1) }; new K().go }
    def viaDef(): String = { class K { def go: String = helper(2) }; new K().go }
    viaLazy + ", " + viaDef()

  def caseClass(): String =
    val suffix = "!"
    lazy val l: String = "case" + suffix
    case class C(i: Int) { def show: String = l + i }
    val c = C(1)
    c.show + " " + c.copy(i = 2).show

  def localObject(): String =
    var made = 0
    object O { made += 1; val v = "local object" }
    O.v + " " + O.v + " made " + made

  def main(args: Array[String]): Unit =
    println(lambdaOfPrimitive())
    println(localClass())
    println(localTrait())
    println(s"${mutual(10)} ${mutual(7)}")
    println(varInInitialiser())
    println(makesLocalClass())
    println(caseClass())
    println(localObject())
