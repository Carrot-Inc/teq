// Vals and vars with initialisers in traits, mixed into a class and an object: the initialisers
// run with the trait's statements in order when the class or object is constructed, a private
// val beside an inherited member of its name, a val overridden by the class and by a trait
// below, a val that reads an abstract one the class defines, a var assigned from outside and
// from a trait method, and a lazy val beside them.
trait Base:
  println("Base start")
  val a: Int = 1
  var b: String = "b"
  private val hidden: Int = a + 10
  val ov: Int = { println("Base ov"); 10 }
  lazy val lz: Int = { println("Base lz"); a + 100 }
  def c: Int
  val usesC: Int = c * 2
  def showHidden: Int = hidden
  def bump(): Unit = b = b + "!"
  println("Base end " + a + " " + b)

trait Over extends Base:
  override val ov: Int = { println("Over ov"); 30 }
  val u: Int = ov + 1

class Plain extends Base:
  def c: Int = 4
  override val ov: Int = 20

class Deep extends Over:
  def c: Int = 5
  val hidden: String = "own"

object Single extends Over:
  def c: Int = 6
  val tail: Int = u + a

object Main:
  def main(args: Array[String]): Unit =
    val p = new Plain
    println(s"${p.a} ${p.b} ${p.ov} ${p.usesC} ${p.showHidden}")
    p.b = "bb"
    p.bump()
    println(p.b)
    println(p.lz)
    println(p.lz)
    val d = new Deep
    println(s"${d.ov} ${d.u} ${d.usesC} ${d.hidden} ${d.showHidden}")
    println(s"${Single.ov} ${Single.u} ${Single.tail} ${Single.usesC}")
    val bs: List[Base] = List(p, d, Single)
    println(bs.map(x => x.a + x.ov).sum)
