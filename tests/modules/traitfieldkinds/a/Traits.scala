package tfka

// Trait fields of the kinds a class over the products reads from the trait's class file, as from a
// jar's: a private var and a private lazy val under their expanded names (`tfka$Kinds$$p`), a final
// val's final getter, a given's and an object's holders, a val the trait's own `$init$` reads that
// a class overrides, and a trait of statements alone. scalac's downstream implements the same accessors.
case class Label(text: String)

trait Kinds:
  val v: Int = { println("init v"); 1 }
  private var p: Int = 10
  private val q: String = "q"
  private lazy val pl: String = { println("init pl"); q + "!" }
  final val fin: Int = 7
  given label: Label = { println("make Label"); Label("k") }
  object Counter:
    var n: Int = 0
  def bumpP(): Int = { p += 1; p }
  def privates: String = s"$q $pl $pl"

trait Over:
  val o: Int = 10
  println("Over init sees " + o)

trait Says:
  println("Says body")
