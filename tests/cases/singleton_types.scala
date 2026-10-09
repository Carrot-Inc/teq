object Registry:
  val name: String = "registry"
  def size: Int = 3

object Css:
  override def toString: String = "Css"

class Counter(val start: Int):
  def same(other: this.type): Boolean = other.start == start
  def self: this.type = this

object Holder:
  val css: Css.type = Css
  val label: String = "holder"
  def relabel(s: label.type): String = s + "!"

def describe(r: Registry.type): String = r.name + r.size

def twin(x: String)(y: x.type): String = x + y

def keep[S <: String & Singleton](s: S): S = s

def stored[T](using v: ValueOf[T]): T = v.value

def lengthOf[T <: Singleton](t: T): String = t.toString

trait Mirror[E]:
  def label: String

object Mirror:
  def apply[E](using mirror: Mirror[E]): mirror.type = mirror
  given Mirror[Int] with
    def label: String = "int mirror"

def wrapped(x: String): Option[x.type] = Some(x)

@main def main(): Unit =
  println(describe(Registry))
  val css: Css.type = Css
  println(css)
  println(Holder.css)
  println(Holder.relabel(Holder.label))
  val word = "ab"
  println(twin(word)(word))
  val c = Counter(5)
  println(c.same(c))
  println(c.self.start)
  println(keep("kept"))
  println(stored[Int](using ValueOf(3)))
  println(lengthOf(Registry.name))
  val same: word.type = word
  println(same)
  println(Mirror.apply[Int].label)
  println(wrapped(word))
