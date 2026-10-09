package object scala2lib {
  type Cell[A] = Encoder[String, A, codecs.type]
  val defaultHeader: List[String] = List("a", "b")

  implicit class Shout(val s: String) extends AnyVal {
    def shout: String = s.toUpperCase + "!"
    def shoutTimes(n: Int = 2): String = List.fill(n)(shout).mkString(" ")
  }

  def describe(s: Shape): String = s match {
    case Square(x) => s"square $x"
    case Dot => "dot"
  }
}

// What a Scala 2 trait's transcoded pickle leaves out and the class mixing it in holds from the trait's class file
// alone (tests/classpath/jvm/scala2_trait_storage): a private var (Spark's `Logging` has `log_`) with its expanded
// accessors, a private lazy val's holder over the static `T$$x$` (a private def, of the same shape, none), an
// object's and a private object's holder made with the instance as the outer reference, and the `$init$` of a trait
// whose initialisers are all private or whose body is statements alone.
trait Logging {
  private var log_ : String = null
  protected def log: String = {
    if (log_ == null) { log_ = "logger"; println("made " + log_) }
    log_
  }
  def logInfo(msg: String): String = log + ": " + msg
}

trait Ticker {
  private var count = 0
  object cell { println("cell init"); val owner: Ticker = Ticker.this; def advance: Int = step }
  private object hidden { println("hidden init"); val h = 4 }
  private lazy val priv: Int = { println("priv init"); 2 }
  private def step: Int = { println("step"); 1 }
  def tick: Int = { count += cell.advance; count + priv + hidden.h }
}

trait Says { println("Says body") }

trait OnlyPrivate {
  private val secret: Int = { println("OnlyPrivate init"); 9 }
  private var hits: Int = 1
  def reveal: Int = { hits += 1; secret + hits }
}

// A trait nested in an object, whose pickle is its top-level class's (`Box.class`), and a private lazy val computed
// to null (tests/classpath/jvm/scala2_nested_trait_storage): the class mixing the trait in holds its private var,
// its object and its lazy val from the class file, read through the owning pickle, and calls its `$init$`.
object Box {
  trait Nested {
    private var count = { println("Nested init"); 1 }
    object cell { println("cell init"); def owner: Nested = Nested.this }
    private lazy val twice: Int = { println("twice init"); count * 2 }
    def next: Int = { count += 1; count + twice }
  }
}

trait NullLazy {
  private lazy val missing: String = { println("missing init"); null }
  def read: String = missing
}
