// A selection, an application or an assignment on a value whose class derives from `scala.Dynamic` and has no such
// member is rewritten as scalac's typer rewrites it (`typer/Dynamic.scala`): `selectDynamic`, `applyDynamic`,
// `applyDynamicNamed`, `updateDynamic`, with the name as a string; a member the class has is taken first.
import scala.language.dynamics

class Echo(val tag: String) extends Dynamic:
  def selectDynamic(name: String): String = s"$tag.select($name)"
  def applyDynamic(name: String)(args: Any*): String = s"$tag.apply($name)(${args.mkString(",")})"
  def applyDynamicNamed(name: String)(args: (String, Any)*): String =
    s"$tag.named($name)(${args.map((k, v) => s"$k=$v").mkString(",")})"
  def updateDynamic(name: String)(value: Any): Unit = println(s"$tag.update($name)($value)")
  def real(n: Int): String = s"$tag.real($n)"
  def viaThis: String = this.inner + "|" + this.call(1)

class Typed extends Dynamic:
  def selectDynamic[T](name: String): Option[T] = None
  def applyDynamic[T](name: String)(args: T*): List[T] = args.toList

class WithUsing extends Dynamic:
  def applyDynamic(name: String)(n: Int, scale: Int = 10)(using label: String): String = s"$label:$name:${n * scale}"

class Chain(val path: String) extends Dynamic:
  def selectDynamic(name: String): Chain = new Chain(s"$path.$name")
  def applyDynamic(name: String)(args: Any*): Chain = new Chain(s"$path.$name(${args.mkString(",")})")
  override def toString: String = path

object Registry extends Dynamic:
  def selectDynamic(name: String): String = s"registry.$name"

enum Level extends Dynamic:
  case Low, High
  def selectDynamic(name: String): String = s"$this.$name"

case class Row(id: Int) extends Dynamic:
  def selectDynamic(name: String): String = s"row$id.$name"

type Aliased = Echo
def bounded[T <: Echo](t: T): String = t.field
def viaAlias(e: Aliased): String = e.aliased

@main def run(): Unit =
  val d = new Echo("d")
  println(d.field)
  println(d.call(1, "a"))
  println(d.call())
  println(d(1))
  println(d())
  println(d + 1)
  println(d ++ List(2))
  println(-d)
  println(d.named(x = 1, y = "b"))
  println(d.mixed(1, y = 2))
  println(d(a = 1))
  println(d.real(3))
  d.slot = 7
  println(d.viaThis)
  val xs = Seq(1, 2)
  println(d.spread(xs*))
  println(s"${d.inside} and ${d.count(2)}")
  val t = new Typed
  println(t.pick[Int])
  println(t.all[String]("a", "b"))
  given String = "lbl"
  println(new WithUsing().scaled(4))
  println(new WithUsing().scaled(4, 2))
  val c = new Chain("c")
  println(c.a.b(1).c)
  println(Registry.entry)
  println(Level.High.rank)
  println(Row(1).name)
  println(bounded(d))
  println(viaAlias(d))
  val s: d.type = d
  println(s.single)
  val n: Echo | Null = d
  println(n.nullable)
  val f: Echo => String = _.lambda
  println(f(d))
  lazy val lz = new Echo("lz")
  println(lz.lazily)
