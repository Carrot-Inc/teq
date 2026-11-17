// An assignment to a name a Dynamic class lacks is `updateDynamic`; an indexed assignment goes through the
// selection's result (`d.items(0) = v` is `d.selectDynamic("items").update(0, v)`) or, on the value itself, through
// `applyDynamic("update")`; a member that is there and cannot be accessed is rewritten too. An assignment operator
// on a `var` of a class without an `applyDynamic` is the assignment by the class's own operator: the application
// as written, `t.+=(1)`, cannot be typed, as under scalac.
import scala.language.dynamics

class Slots extends Dynamic:
  val items: Array[Int] = Array(1, 2)
  def selectDynamic(name: String): Array[Int] = { println(s"select($name)"); items }
  def applyDynamic(name: String)(args: Any*): String = s"apply($name)(${args.mkString(",")})"
  def updateDynamic(name: String)(value: Any): Unit = println(s"update($name)($value)")

class Vault extends Dynamic:
  private var secret: Int = 1
  private def hidden(): String = "hidden"
  protected def guarded: String = "guarded"
  def selectDynamic(name: String): String = s"select($name)"
  def applyDynamic(name: String)(args: Any*): String = s"apply($name)(${args.mkString(",")})"
  def updateDynamic(name: String)(value: Any): Unit = println(s"update($name)($value)")
  def inside: String = s"$secret ${hidden()} $guarded"

class Keeper extends Vault:
  def own: String = guarded

class Sum(val n: Int) extends Dynamic:
  def +(k: Int): Sum = { println(s"plus($k)"); new Sum(n + k) }
  def selectDynamic(name: String): String = s"select($name)"

@main def run(): Unit =
  val s = new Slots
  s.count = 3
  s.other(0) = 9
  println(s.items.toList)
  println(s(0) = 5)
  val v = new Vault
  v.secret = 2
  println(v.secret)
  println(v.hidden())
  println(v.guarded)
  println(v.inside)
  println(new Keeper().own)
  var t = new Sum(1)
  t += 1
  println(t.n)
