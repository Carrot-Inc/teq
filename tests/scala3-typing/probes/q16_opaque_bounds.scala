object Ids:
  opaque type Pos <: Int = Int
  object Pos:
    def apply(i: Int): Pos = i
  opaque type Email = String
  object Email:
    def apply(s: String): Email = s
    extension (e: Email) def domain: String = e.split("@")(1)
object Use:
  import Ids.*
  val p: Pos = 3
  val q: Int = Pos(3) + 1
  val e: Email = "a@b"
  val s: String = Email("a@b")
  def len(e: Email) = e.length
  val t: Email = Email("x@y").domain
@main def run(): Unit = println(Use.q)
